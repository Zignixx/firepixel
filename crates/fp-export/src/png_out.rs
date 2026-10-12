//! PNG-Schreiber (08 §5.1): PNG8 (Palette, Bittiefe 1/2/4/8 nach Palettengröße, `tRNS`), PNG24, PNG32,
//! `sBIT`, `pHYs`, `tEXt`, Adam7-Zeilensprung und APNG (animiertes PNG, `acTL`/`fcTL`/`fdAT`).
//!
//! Chunk-Reihenfolge wie das Original: `IHDR sBIT PLTE tRNS pHYs tEXt… IDAT IEND` (Laufzeit 08 T2).
//! Eigene Zeilenfilter (indiziert: keiner; Truecolor: adaptiv nach kleinster Betragssumme wie libpng)
//! und zlib über `flate2`.

use std::io::Write;

use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::pipeline::{ExportContext, IndexedImage};
use crate::px;
use crate::{ExportError, Result};

/// Optionen des PNG-Schreibers.
#[derive(Clone, Debug, PartialEq)]
pub struct PngOptions {
    /// Adam7-Zeilensprung.
    pub interlaced: bool,
    /// Auflösung in dpi (`pHYs` = dpi·10000/254 Pixel/m), `None` = kein `pHYs`.
    pub dpi: Option<f32>,
    /// `tEXt`-Einträge (Schlüssel, Latin-1-Text).
    pub text: Vec<(String, String)>,
    /// zlib-Stufe 0…9.
    pub level: u32,
}

impl Default for PngOptions {
    fn default() -> Self {
        PngOptions { interlaced: false, dpi: Some(72.0), text: Vec::new(), level: 6 }
    }
}

impl PngOptions {
    /// Optionen aus dem Export-Kontext (Software-Text, Erstellungszeit, Auflösung).
    pub fn from_context(ctx: &ExportContext, interlaced: bool) -> PngOptions {
        let mut text = Vec::new();
        if let Some(s) = &ctx.software {
            text.push(("Software".to_string(), s.clone()));
        }
        if let Some(t) = &ctx.creation_time {
            text.push(("Creation Time".to_string(), t.clone()));
        }
        PngOptions { interlaced: interlaced || ctx.png_adam7, dpi: Some(ctx.resolution_dpi), text, level: 6 }
    }
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32fast::hash(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

fn latin1(s: &str) -> Vec<u8> {
    s.chars().map(|c| if (c as u32) < 256 { c as u8 } else { b'?' }).collect()
}

/// Pixelformat der Rohzeilen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// Palette mit Bittiefe.
    Indexed(u8),
    Rgb,
    Rgba,
}

impl Kind {
    fn bpp(self) -> usize {
        match self {
            Kind::Indexed(_) => 1,
            Kind::Rgb => 3,
            Kind::Rgba => 4,
        }
    }
}

/// Packt eine Zeile (Indizes bzw. RGB(A)-Bytes) in Rohbytes.
fn pack_row(kind: Kind, row: &[u8]) -> Vec<u8> {
    match kind {
        Kind::Indexed(bits) if bits < 8 => {
            let per = 8 / bits as usize;
            let mut out = vec![0u8; row.len().div_ceil(per)];
            for (i, &v) in row.iter().enumerate() {
                let shift = 8 - bits as usize * (i % per + 1);
                out[i / per] |= (v & ((1u8 << bits) - 1)) << shift;
            }
            out
        }
        _ => row.to_vec(),
    }
}

fn filter_rows(kind: Kind, rows: &[Vec<u8>]) -> Vec<u8> {
    let bpp = kind.bpp().max(1);
    let mut out = Vec::new();
    let mut prev: Vec<u8> = Vec::new();
    for raw in rows {
        if prev.len() != raw.len() {
            prev = vec![0; raw.len()];
        }
        if matches!(kind, Kind::Indexed(_)) {
            out.push(0);
            out.extend_from_slice(raw);
        } else {
            let mut best: Option<(u64, u8, Vec<u8>)> = None;
            for ft in 0..5u8 {
                let mut f = vec![0u8; raw.len()];
                for i in 0..raw.len() {
                    let a = if i >= bpp { raw[i - bpp] } else { 0 } as i16;
                    let b = prev[i] as i16;
                    let c = if i >= bpp { prev[i - bpp] } else { 0 } as i16;
                    let pred = match ft {
                        0 => 0,
                        1 => a,
                        2 => b,
                        3 => (a + b) / 2,
                        _ => {
                            let p = a + b - c;
                            let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
                            if pa <= pb && pa <= pc {
                                a
                            } else if pb <= pc {
                                b
                            } else {
                                c
                            }
                        }
                    };
                    f[i] = (raw[i] as i16 - pred) as u8;
                }
                let sum: u64 = f.iter().map(|&v| (v as i8).unsigned_abs() as u64).sum();
                if best.as_ref().is_none_or(|bst| sum < bst.0) {
                    best = Some((sum, ft, f));
                }
            }
            if let Some((_, ft, f)) = best {
                out.push(ft);
                out.extend_from_slice(&f);
            }
        }
        prev = raw.clone();
    }
    out
}

/// Bildzeilen (ungepackt: Indizes bzw. Bytes je Pixel) → gefilterte, ggf. Adam7-geordnete Daten.
fn image_data(kind: Kind, w: usize, h: usize, rows: &[Vec<u8>], interlaced: bool) -> Vec<u8> {
    let bpp = kind.bpp();
    if !interlaced {
        let packed: Vec<Vec<u8>> = rows.iter().map(|r| pack_row(kind, r)).collect();
        return filter_rows(kind, &packed);
    }
    const PASSES: [(usize, usize, usize, usize); 7] = [(0, 0, 8, 8), (4, 0, 8, 8), (0, 4, 4, 8), (2, 0, 4, 4), (0, 2, 2, 4), (1, 0, 2, 2), (0, 1, 1, 2)];
    let mut out = Vec::new();
    for (x0, y0, dx, dy) in PASSES {
        if x0 >= w || y0 >= h {
            continue;
        }
        let mut pass_rows = Vec::new();
        for y in (y0..h).step_by(dy) {
            let mut r = Vec::new();
            for x in (x0..w).step_by(dx) {
                r.extend_from_slice(&rows[y][x * bpp..x * bpp + bpp]);
            }
            pass_rows.push(pack_row(kind, &r));
        }
        out.extend(filter_rows(kind, &pass_rows));
    }
    out
}

fn deflate(data: &[u8], level: u32) -> Result<Vec<u8>> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::new(level.min(9)));
    e.write_all(data)?;
    Ok(e.finish()?)
}

fn header(out: &mut Vec<u8>, w: usize, h: usize, kind: Kind, interlaced: bool) -> Result<()> {
    if w == 0 || h == 0 || w > u32::MAX as usize / 8 || h > u32::MAX as usize / 8 {
        return Err(ExportError::BadSize(w, h));
    }
    out.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    let (depth, ctype) = match kind {
        Kind::Indexed(b) => (b, 3u8),
        Kind::Rgb => (8, 2),
        Kind::Rgba => (8, 6),
    };
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[depth, ctype, 0, 0, u8::from(interlaced)]);
    chunk(out, b"IHDR", &ihdr);
    let sbit: &[u8] = match kind {
        Kind::Rgba => &[8, 8, 8, 8],
        _ => &[8, 8, 8],
    };
    chunk(out, b"sBIT", sbit);
    Ok(())
}

fn trailer_meta(out: &mut Vec<u8>, opts: &PngOptions) {
    if let Some(dpi) = opts.dpi
        && dpi.is_finite()
        && dpi > 0.0
    {
        // Das Original schneidet ab (72 dpi -> 2834 Pixel/m, `re` §5.2); das kleine
        // Epsilon fängt Werte wie 254 dpi ab, die rechnerisch knapp unter der Ganzzahl landen.
        let ppm = (dpi as f64 * 10000.0 / 254.0 + 1e-6).floor() as u32;
        let mut d = Vec::with_capacity(9);
        d.extend_from_slice(&ppm.to_be_bytes());
        d.extend_from_slice(&ppm.to_be_bytes());
        d.push(1);
        chunk(out, b"pHYs", &d);
    }
    for (k, v) in &opts.text {
        let mut d = latin1(k);
        d.truncate(79);
        d.push(0);
        d.extend(latin1(v));
        chunk(out, b"tEXt", &d);
    }
}

/// Bittiefe für eine Palettengröße (≤2 → 1, ≤4 → 2, ≤16 → 4, sonst 8).
pub fn indexed_depth(n: usize) -> u8 {
    match n {
        0..=2 => 1,
        3..=4 => 2,
        5..=16 => 4,
        _ => 8,
    }
}

fn palette_chunks(out: &mut Vec<u8>, img: &IndexedImage, matte: u32) {
    let pal = img.palette_rgb(matte);
    let mut plte = Vec::with_capacity(pal.len() * 3);
    for c in &pal {
        plte.extend_from_slice(c);
    }
    chunk(out, b"PLTE", &plte);
    let mut alphas: Vec<u8> = (0..pal.len())
        .map(|i| {
            if Some(i) == img.transparent_index {
                0
            } else {
                img.palette.get(i).map(|&c| px::a(c) as u8).unwrap_or(0)
            }
        })
        .collect();
    while alphas.last() == Some(&255) {
        alphas.pop();
    }
    if !alphas.is_empty() {
        chunk(out, b"tRNS", &alphas);
    }
}

/// PNG8 aus einem Indexbild.
pub fn write_png8(img: &IndexedImage, matte: u32, opts: &PngOptions) -> Result<Vec<u8>> {
    let n = img.file_palette_len().max(1);
    let kind = Kind::Indexed(indexed_depth(n));
    let mut out = Vec::new();
    header(&mut out, img.width, img.height, kind, opts.interlaced)?;
    palette_chunks(&mut out, img, matte);
    trailer_meta(&mut out, opts);
    let rows: Vec<Vec<u8>> = img.indices.chunks(img.width.max(1)).take(img.height).map(|r| r.to_vec()).collect();
    let data = deflate(&image_data(kind, img.width, img.height, &rows, opts.interlaced), opts.level)?;
    chunk(&mut out, b"IDAT", &data);
    chunk(&mut out, b"IEND", &[]);
    Ok(out)
}

fn rows_truecolor(pixels: &[u32], w: usize, h: usize, alpha: bool) -> Vec<Vec<u8>> {
    (0..h)
        .map(|y| {
            let mut r = Vec::with_capacity(w * 4);
            for &c in &pixels[y * w..(y + 1) * w] {
                r.extend_from_slice(&[px::r(c) as u8, px::g(c) as u8, px::b(c) as u8]);
                if alpha {
                    r.push(px::a(c) as u8);
                }
            }
            r
        })
        .collect()
}

/// PNG24 (`alpha = false`, Pixel bereits über die Matte gemischt) bzw. PNG32.
pub fn write_png_truecolor(pixels: &[u32], w: usize, h: usize, alpha: bool, opts: &PngOptions) -> Result<Vec<u8>> {
    if pixels.len() < w * h {
        return Err(ExportError::BadSize(w, h));
    }
    let kind = if alpha { Kind::Rgba } else { Kind::Rgb };
    let mut out = Vec::new();
    header(&mut out, w, h, kind, opts.interlaced)?;
    trailer_meta(&mut out, opts);
    let data = deflate(&image_data(kind, w, h, &rows_truecolor(pixels, w, h, alpha), opts.interlaced), opts.level)?;
    chunk(&mut out, b"IDAT", &data);
    chunk(&mut out, b"IEND", &[]);
    Ok(out)
}

/// Ein Einzelbild eines APNG (Pixel `0xAARRGGBB` im Rechteck `x, y, w, h` der Leinwand).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApngFrame {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub pixels: Vec<u32>,
    /// Verzögerung in 1/100 s.
    pub delay_cs: u16,
    /// `dispose_op`: 0 keine, 1 Hintergrund (transparent), 2 vorheriges.
    pub dispose: u8,
    /// `blend_op`: 0 ersetzen, 1 über das Vorherige.
    pub blend: u8,
}

/// APNG (RGBA, 8 Bit) mit `num_plays` Wiederholungen (0 = endlos). Das erste Einzelbild muss die ganze
/// Leinwand abdecken (es ist zugleich das Standardbild).
pub fn write_apng(width: u32, height: u32, frames: &[ApngFrame], num_plays: u32, opts: &PngOptions) -> Result<Vec<u8>> {
    let first = frames.first().ok_or_else(|| ExportError::Encode(fp_i18n::t("err-export-d-apng-no-frame")))?;
    if first.x != 0 || first.y != 0 || first.w != width || first.h != height {
        return Err(ExportError::Encode(fp_i18n::t("err-export-d-apng-first-frame")));
    }
    let mut out = Vec::new();
    header(&mut out, width as usize, height as usize, Kind::Rgba, false)?;
    let mut actl = Vec::new();
    actl.extend_from_slice(&(frames.len() as u32).to_be_bytes());
    actl.extend_from_slice(&num_plays.to_be_bytes());
    chunk(&mut out, b"acTL", &actl);
    trailer_meta(&mut out, opts);
    let mut seq = 0u32;
    for (i, f) in frames.iter().enumerate() {
        if f.pixels.len() < (f.w * f.h) as usize || f.w == 0 || f.h == 0 || f.x + f.w > width || f.y + f.h > height {
            return Err(ExportError::BadSize(f.w as usize, f.h as usize));
        }
        let mut fctl = Vec::with_capacity(26);
        fctl.extend_from_slice(&seq.to_be_bytes());
        seq += 1;
        for v in [f.w, f.h, f.x, f.y] {
            fctl.extend_from_slice(&v.to_be_bytes());
        }
        fctl.extend_from_slice(&f.delay_cs.to_be_bytes());
        fctl.extend_from_slice(&100u16.to_be_bytes());
        fctl.extend_from_slice(&[f.dispose.min(2), f.blend.min(1)]);
        chunk(&mut out, b"fcTL", &fctl);
        let rows = rows_truecolor(&f.pixels, f.w as usize, f.h as usize, true);
        let data = deflate(&image_data(Kind::Rgba, f.w as usize, f.h as usize, &rows, false), opts.level)?;
        if i == 0 {
            chunk(&mut out, b"IDAT", &data);
        } else {
            let mut d = Vec::with_capacity(data.len() + 4);
            d.extend_from_slice(&seq.to_be_bytes());
            seq += 1;
            d.extend_from_slice(&data);
            chunk(&mut out, b"fdAT", &d);
        }
    }
    chunk(&mut out, b"IEND", &[]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(bytes: &[u8]) -> (png::OutputInfo, Vec<u8>) {
        let mut d = png::Decoder::new(std::io::Cursor::new(bytes));
        d.set_transformations(png::Transformations::EXPAND);
        let mut r = d.read_info().unwrap();
        let mut buf = vec![0; r.output_buffer_size().unwrap()];
        let info = r.next_frame(&mut buf).unwrap();
        (info, buf)
    }

    #[test]
    fn truecolor_and_interlace_roundtrip() {
        let (w, h) = (13usize, 9usize);
        let px: Vec<u32> = (0..w * h).map(|i| px::argb((i * 7) as u32 & 255, i as u32 & 255, (i * 3) as u32 & 255, 200)).collect();
        for il in [false, true] {
            let png = write_png_truecolor(&px, w, h, true, &PngOptions { interlaced: il, ..Default::default() }).unwrap();
            let (info, buf) = decode(&png);
            assert_eq!((info.width, info.height), (13, 9));
            for i in 0..w * h {
                assert_eq!(px::from_rgba([buf[i * 4], buf[i * 4 + 1], buf[i * 4 + 2], buf[i * 4 + 3]]), px[i]);
            }
        }
    }

    #[test]
    fn png8_depths() {
        let img = IndexedImage {
            width: 5,
            height: 3,
            palette: vec![0xFFFF0000, 0x0000FF00, 0xFF0000FF],
            indices: vec![0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2],
            transparent_index: None,
            counts: vec![],
            alpha_palette: true,
        };
        let png = write_png8(&img, 0xFFFFFFFF, &PngOptions::default()).unwrap();
        let mut d = png::Decoder::new(std::io::Cursor::new(&png));
        d.set_transformations(png::Transformations::EXPAND);
        let mut r = d.read_info().unwrap();
        assert_eq!(r.info().bit_depth, png::BitDepth::Two);
        let mut buf = vec![0; r.output_buffer_size().unwrap()];
        r.next_frame(&mut buf).unwrap();
        assert_eq!(&buf[0..8], &[255, 0, 0, 255, 0, 255, 0, 0]);
    }

    #[test]
    fn apng_frames() {
        let f0 = ApngFrame { x: 0, y: 0, w: 2, h: 2, pixels: vec![0xFFFF0000; 4], delay_cs: 7, dispose: 0, blend: 0 };
        let f1 = ApngFrame { x: 1, y: 1, w: 1, h: 1, pixels: vec![0xFF00FF00], delay_cs: 7, dispose: 0, blend: 1 };
        let png = write_apng(2, 2, &[f0, f1], 0, &PngOptions::default()).unwrap();
        let d = png::Decoder::new(std::io::Cursor::new(&png));
        let r = d.read_info().unwrap();
        assert_eq!(r.info().animation_control.map(|a| a.num_frames), Some(2));
    }
}
