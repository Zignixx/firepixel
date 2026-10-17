//! Weitere Bildformate (08 §5.4 und Erweiterungen): TIFF, BMP, WBMP, QOI und ICO (mehrere Größen).
//!
//! Das Original schreibt TIFF/BMP/WBMP über Photoshop-artige Plug-ins (`exportFormat = 3`); deren genaue
//! Bytefolge ist nicht spezifiziert. Hier entstehen schlichte, normgerechte Dateien:
//! * TIFF: Baseline, Little Endian, ein Streifen, unkomprimiert oder PackBits; indiziert (Palette),
//!   RGB oder RGBA (nicht zugeordnetes Alpha `ExtraSamples = 2`) und Auflösung.
//! * BMP: 8 Bit mit Palette, 24 Bit oder 32 Bit (wie das Original immer mit 40-Byte-`BITMAPINFOHEADER`,
//!   re\27 E5), Zeilen von unten.
//! * WBMP: Typ 0, 1 Bit (weiß = 1).
//! * QOI: vollständiger Kodierer nach der QOI-Spezifikation.
//! * ICO: je Größe ein Eintrag; ≥ 256 px als PNG, sonst 32-Bit-DIB mit AND-Maske.

use crate::pipeline::IndexedImage;
use crate::png_out::{PngOptions, write_png_truecolor};
use crate::px::{a, b, g, lum, r};
use crate::{ExportError, Result};

// ---------------------------------------------------------------- TIFF
/// Kompression des TIFF-Schreibers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TiffCompression {
    #[default]
    None,
    PackBits,
}

/// PackBits-Kodierung einer Zeile (TIFF/PSD).
pub fn packbits(row: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(row.len() + row.len() / 64 + 2);
    let n = row.len();
    let mut i = 0;
    while i < n {
        let mut run = 1;
        while i + run < n && run < 128 && row[i + run] == row[i] {
            run += 1;
        }
        if run >= 2 {
            out.push((257 - run) as u8);
            out.push(row[i]);
            i += run;
            continue;
        }
        let start = i;
        let mut lit = 0;
        while i < n && lit < 128 {
            if i + 1 < n && row[i] == row[i + 1] && (i + 2 >= n || row[i + 1] == row[i + 2]) {
                break;
            }
            i += 1;
            lit += 1;
        }
        if lit == 0 {
            i += 1;
            lit = 1;
        }
        out.push((lit - 1) as u8);
        out.extend_from_slice(&row[start..start + lit]);
    }
    out
}

/// Bildinhalt für TIFF/BMP.
pub enum RasterData<'a> {
    /// Indexbild.
    Indexed(&'a IndexedImage),
    /// Truecolor (`0xAARRGGBB`), `alpha` = Alphakanal schreiben.
    Truecolor {
        width: usize,
        height: usize,
        pixels: &'a [u32],
        alpha: bool,
    },
}

impl RasterData<'_> {
    fn dims(&self) -> (usize, usize) {
        match self {
            RasterData::Indexed(i) => (i.width, i.height),
            RasterData::Truecolor { width, height, .. } => (*width, *height),
        }
    }
}

/// Baseline-TIFF (Little Endian, ein Streifen).
pub fn write_tiff(
    data: &RasterData,
    dpi: f32,
    compression: TiffCompression,
    software: Option<&str>,
) -> Result<Vec<u8>> {
    let (w, h) = data.dims();
    if w == 0 || h == 0 || w > u32::MAX as usize / 4 {
        return Err(ExportError::BadSize(w, h));
    }
    let (spp, photometric, rows): (u16, u16, Vec<Vec<u8>>) = match data {
        RasterData::Indexed(img) => (
            1,
            3,
            img.indices.chunks(w).take(h).map(|r| r.to_vec()).collect(),
        ),
        RasterData::Truecolor { pixels, alpha, .. } => {
            let spp = if *alpha { 4 } else { 3 };
            let rows = (0..h)
                .map(|y| {
                    let mut row = Vec::with_capacity(w * spp);
                    for &c in &pixels[y * w..(y + 1) * w] {
                        row.extend_from_slice(&[r(c) as u8, g(c) as u8, b(c) as u8]);
                        if *alpha {
                            row.push(a(c) as u8);
                        }
                    }
                    row
                })
                .collect();
            (spp as u16, 2, rows)
        }
    };
    let strip: Vec<u8> = match compression {
        TiffCompression::None => rows.concat(),
        TiffCompression::PackBits => rows.iter().flat_map(|r| packbits(r)).collect(),
    };
    let mut out = b"II*\0\0\0\0\0".to_vec();
    let strip_off = out.len() as u32;
    out.extend_from_slice(&strip);
    if out.len() % 2 == 1 {
        out.push(0);
    }
    // Zusatzdaten (BitsPerSample, Auflösung, Farbtabelle, Software)
    let mut extra: Vec<u8> = Vec::new();
    let base = out.len() as u32;
    let mut add = |bytes: &[u8]| -> u32 {
        let off = base + extra.len() as u32;
        extra.extend_from_slice(bytes);
        if extra.len() % 2 == 1 {
            extra.push(0);
        }
        off
    };
    let bps_off = if spp > 2 {
        add(&[8u16; 4][..spp as usize]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<u8>>())
    } else {
        0
    };
    let res = (if dpi.is_finite() && dpi > 0.0 {
        dpi
    } else {
        72.0
    } * 10000.0)
        .round() as u32;
    let xres = add(&[res.to_le_bytes(), 10000u32.to_le_bytes()].concat());
    let yres = add(&[res.to_le_bytes(), 10000u32.to_le_bytes()].concat());
    let cmap = if let RasterData::Indexed(img) = data {
        let mut t = vec![0u16; 768];
        for (i, &c) in img.palette.iter().take(256).enumerate() {
            t[i] = (r(c) * 257) as u16;
            t[256 + i] = (g(c) * 257) as u16;
            t[512 + i] = (b(c) * 257) as u16;
        }
        Some(add(&t
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<u8>>()))
    } else {
        None
    };
    let soft = software.map(|s| {
        let mut v: Vec<u8> = s.bytes().filter(|c| c.is_ascii()).collect();
        v.push(0);
        if v.len() <= 4 {
            let mut inline = [0u8; 4];
            inline[..v.len()].copy_from_slice(&v);
            (v.len() as u32, u32::from_le_bytes(inline))
        } else {
            (v.len() as u32, add(&v))
        }
    });
    out.extend_from_slice(&extra);
    let ifd_off = out.len() as u32;
    out[4..8].copy_from_slice(&ifd_off.to_le_bytes());
    let mut entries: Vec<(u16, u16, u32, u32)> = vec![
        (256, 4, 1, w as u32),
        (257, 4, 1, h as u32),
        if spp > 2 {
            (258, 3, spp as u32, bps_off)
        } else {
            (258, 3, 1, 8)
        },
        (
            259,
            3,
            1,
            if compression == TiffCompression::PackBits {
                32773
            } else {
                1
            },
        ),
        (262, 3, 1, photometric as u32),
        (273, 4, 1, strip_off),
        (277, 3, 1, spp as u32),
        (278, 4, 1, h as u32),
        (279, 4, 1, strip.len() as u32),
        (282, 5, 1, xres),
        (283, 5, 1, yres),
        (284, 3, 1, 1),
        (296, 3, 1, 2),
    ];
    if let Some((n, off)) = soft {
        entries.push((305, 2, n, off));
    }
    if let Some(off) = cmap {
        entries.push((320, 3, 768, off));
    }
    if spp == 4 {
        entries.push((338, 3, 1, 2));
    }
    entries.sort_by_key(|e| e.0);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    for (tag, typ, count, val) in entries {
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&typ.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        if typ == 3 && count == 1 {
            out.extend_from_slice(&(val as u16).to_le_bytes());
            out.extend_from_slice(&[0, 0]);
        } else {
            out.extend_from_slice(&val.to_le_bytes());
        }
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    Ok(out)
}

// ---------------------------------------------------------------- BMP
/// `biXPelsPerMeter`/`biYPelsPerMeter` wie das BMP-Plug-in des Originals (re\27 E5, B):
/// `floor(dpi / 2,5406 · 100 + 0,5)` mit ganzzahligen (abgeschnittenen) dpi; Konstante 2,5406, nicht
/// 2,54 (72 → 2834, 96 → 3779, 150 → 5904, 300 → 11808). Absichtlich nicht die pHYs-Formel des PNG.
pub fn bmp_pels_per_meter(dpi: f32) -> u32 {
    let dpi = if dpi.is_finite() && dpi > 0.0 {
        dpi
    } else {
        72.0
    };
    ((dpi as i32) as f64 / 2.5406 * 100.0 + 0.5)
        .floor()
        .max(0.0) as u32
}

/// Gegenstück beim Lesen (re\27 E5, B): `floor(ppm / 100 · 2,5406 + 0,5)`; `None` bei ppm = 0.
pub fn bmp_dpi_from_ppm(ppm: u32) -> Option<u32> {
    (ppm != 0).then(|| (ppm as f64 / 100.0 * 2.5406 + 0.5).floor() as u32)
}

/// BMP: 8 Bit (Palette), 24 Bit oder 32 Bit mit Alpha. Kopf immer `BITMAPINFOHEADER` (40 Byte,
/// `BI_RGB`), auch bei 32 Bit; bei 8 Bit `biClrUsed` = 256 und `biClrImportant` = Farbanzahl
/// (re\27 E5, B).
pub fn write_bmp(data: &RasterData, dpi: f32) -> Result<Vec<u8>> {
    write_bmp_ex(data, dpi, false)
}

/// Wie [`write_bmp`]; `rle8` = Bittiefe-Modus 1 des BMP-Plug-ins (re E5, B): 8 Bit mit
/// `biCompression` = 1 (`BI_RLE8`). Wirkt nur bei Indexbildern; Truecolor bleibt unkomprimiert.
pub fn write_bmp_ex(data: &RasterData, dpi: f32, rle8: bool) -> Result<Vec<u8>> {
    let (w, h) = data.dims();
    if let (true, RasterData::Indexed(img)) = (rle8, data) {
        if w == 0 || h == 0 || w > i32::MAX as usize || h > i32::MAX as usize {
            return Err(ExportError::BadSize(w, h));
        }
        return Ok(write_bmp_rle8(img, bmp_pels_per_meter(dpi)));
    }
    if w == 0 || h == 0 || w > i32::MAX as usize / 4 || h > i32::MAX as usize {
        return Err(ExportError::BadSize(w, h));
    }
    let ppm = bmp_pels_per_meter(dpi);
    let (bpp, ncolors, important): (u16, usize, u32) = match data {
        RasterData::Indexed(img) => (8, 256, img.palette.len().min(256) as u32),
        RasterData::Truecolor { alpha: false, .. } => (24, 0, 0),
        RasterData::Truecolor { alpha: true, .. } => (32, 0, 0),
    };
    let hdr_size: u32 = 40;
    let stride = (w * bpp as usize / 8).div_ceil(4) * 4;
    let img_size = stride * h;
    let off = 14 + hdr_size as usize + ncolors * 4;
    let mut out = Vec::with_capacity(off + img_size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&((off + img_size) as u32).to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&(off as u32).to_le_bytes());
    out.extend_from_slice(&hdr_size.to_le_bytes());
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(h as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&bpp.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(img_size as u32).to_le_bytes());
    out.extend_from_slice(&ppm.to_le_bytes());
    out.extend_from_slice(&ppm.to_le_bytes());
    out.extend_from_slice(&(ncolors as u32).to_le_bytes());
    out.extend_from_slice(&important.to_le_bytes());
    if let RasterData::Indexed(img) = data {
        for i in 0..256 {
            let c = img.palette.get(i).copied().unwrap_or(0);
            out.extend_from_slice(&[b(c) as u8, g(c) as u8, r(c) as u8, 0]);
        }
    }
    for y in (0..h).rev() {
        let start = out.len();
        match data {
            RasterData::Indexed(img) => out.extend_from_slice(&img.indices[y * w..(y + 1) * w]),
            RasterData::Truecolor { pixels, alpha, .. } => {
                for &c in &pixels[y * w..(y + 1) * w] {
                    out.extend_from_slice(&[b(c) as u8, g(c) as u8, r(c) as u8]);
                    if *alpha {
                        out.push(a(c) as u8);
                    }
                }
            }
        }
        out.resize(start + stride, 0);
    }
    Ok(out)
}

/// 8-Bit-BMP mit `BI_RLE8`: Kopf wie unkomprimiert (40 Byte, `biClrUsed` 256, `biClrImportant` =
/// Farbanzahl), `biSizeImage` = Länge der kodierten Daten.
fn write_bmp_rle8(img: &IndexedImage, ppm: u32) -> Vec<u8> {
    let (w, h) = (img.width, img.height);
    let mut px = Vec::new();
    for y in (0..h).rev() {
        rle8_row(&img.indices[y * w..(y + 1) * w], &mut px);
        px.extend_from_slice(if y == 0 { &[0, 1] } else { &[0, 0] });
    }
    let off = 14 + 40 + 256 * 4;
    let mut out = Vec::with_capacity(off + px.len());
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&((off + px.len()) as u32).to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&(off as u32).to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(h as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&8u16.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&(px.len() as u32).to_le_bytes());
    out.extend_from_slice(&ppm.to_le_bytes());
    out.extend_from_slice(&ppm.to_le_bytes());
    out.extend_from_slice(&256u32.to_le_bytes());
    out.extend_from_slice(&(img.palette.len().min(256) as u32).to_le_bytes());
    for i in 0..256 {
        let c = img.palette.get(i).copied().unwrap_or(0);
        out.extend_from_slice(&[b(c) as u8, g(c) as u8, r(c) as u8, 0]);
    }
    out.extend_from_slice(&px);
    out
}

/// Eine Zeile `BI_RLE8`: Läufe ≥ 2 kodiert (Anzahl, Index), sonst absolute Folgen (0, n ≥ 3, Daten,
/// auf gerade Länge aufgefüllt); einzelne/zwei Restpixel als Läufe der Länge 1.
fn rle8_row(row: &[u8], out: &mut Vec<u8>) {
    let run_at = |i: usize| {
        let mut n = 1;
        while i + n < row.len() && n < 255 && row[i + n] == row[i] {
            n += 1;
        }
        n
    };
    let mut i = 0;
    while i < row.len() {
        let n = run_at(i);
        if n >= 2 {
            out.extend_from_slice(&[n as u8, row[i]]);
            i += n;
            continue;
        }
        // Literalfolge bis zum nächsten Lauf ≥ 2 (höchstens 255)
        let mut j = i;
        while j < row.len() && j - i < 255 && (j == i || run_at(j) < 2) {
            j += 1;
        }
        let lit = &row[i..j];
        if lit.len() >= 3 {
            out.extend_from_slice(&[0, lit.len() as u8]);
            out.extend_from_slice(lit);
            if lit.len() % 2 == 1 {
                out.push(0);
            }
        } else {
            for &v in lit {
                out.extend_from_slice(&[1, v]);
            }
        }
        i = j;
    }
}

// ---------------------------------------------------------------- WBMP
fn write_mbi(out: &mut Vec<u8>, mut v: u32) {
    let mut bytes = vec![(v & 0x7F) as u8];
    v >>= 7;
    while v > 0 {
        bytes.push(((v & 0x7F) as u8) | 0x80);
        v >>= 7;
    }
    bytes.reverse();
    out.extend_from_slice(&bytes);
}

/// WBMP Typ 0 aus einem Indexbild (Palettenfarbe heller als 50 % ⇒ weiß).
pub fn write_wbmp(img: &IndexedImage) -> Result<Vec<u8>> {
    let (w, h) = (img.width, img.height);
    if w == 0 || h == 0 {
        return Err(ExportError::BadSize(w, h));
    }
    let mut out = vec![0, 0];
    write_mbi(&mut out, w as u32);
    write_mbi(&mut out, h as u32);
    let white: Vec<bool> = (0..256)
        .map(|i| {
            img.palette
                .get(i)
                .is_none_or(|&c| a(c) == 0 || lum(c) >= 128)
        })
        .collect();
    for y in 0..h {
        let mut row = vec![0u8; w.div_ceil(8)];
        for x in 0..w {
            if white[img.indices[y * w + x] as usize] {
                row[x / 8] |= 0x80 >> (x % 8);
            }
        }
        out.extend_from_slice(&row);
    }
    Ok(out)
}

// ---------------------------------------------------------------- QOI
/// QOI-Kodierer (RGBA, sRGB).
pub fn write_qoi(pixels: &[u32], w: usize, h: usize, alpha: bool) -> Result<Vec<u8>> {
    if w == 0 || h == 0 || pixels.len() < w * h || w > u32::MAX as usize || h > u32::MAX as usize {
        return Err(ExportError::BadSize(w, h));
    }
    let mut out = b"qoif".to_vec();
    out.extend_from_slice(&(w as u32).to_be_bytes());
    out.extend_from_slice(&(h as u32).to_be_bytes());
    out.push(if alpha { 4 } else { 3 });
    out.push(0);
    let mut index = [[0u8; 4]; 64];
    let mut prev = [0u8, 0, 0, 255];
    let mut run = 0u8;
    let n = w * h;
    for (i, &c) in pixels[..n].iter().enumerate() {
        let px = [
            r(c) as u8,
            g(c) as u8,
            b(c) as u8,
            if alpha { a(c) as u8 } else { 255 },
        ];
        if px == prev {
            run += 1;
            if run == 62 || i == n - 1 {
                out.push(0xC0 | (run - 1));
                run = 0;
            }
            continue;
        }
        if run > 0 {
            out.push(0xC0 | (run - 1));
            run = 0;
        }
        let hash =
            (px[0] as usize * 3 + px[1] as usize * 5 + px[2] as usize * 7 + px[3] as usize * 11)
                % 64;
        if index[hash] == px {
            out.push(hash as u8);
        } else {
            index[hash] = px;
            if px[3] == prev[3] {
                let dr = px[0].wrapping_sub(prev[0]) as i8;
                let dg = px[1].wrapping_sub(prev[1]) as i8;
                let db = px[2].wrapping_sub(prev[2]) as i8;
                let (dr_dg, db_dg) = (dr.wrapping_sub(dg), db.wrapping_sub(dg));
                if (-2..=1).contains(&dr) && (-2..=1).contains(&dg) && (-2..=1).contains(&db) {
                    out.push(
                        0x40 | (((dr + 2) as u8) << 4) | (((dg + 2) as u8) << 2) | (db + 2) as u8,
                    );
                } else if (-32..=31).contains(&dg)
                    && (-8..=7).contains(&dr_dg)
                    && (-8..=7).contains(&db_dg)
                {
                    out.push(0x80 | (dg + 32) as u8);
                    out.push((((dr_dg + 8) as u8) << 4) | (db_dg + 8) as u8);
                } else {
                    out.extend_from_slice(&[0xFE, px[0], px[1], px[2]]);
                }
            } else {
                out.extend_from_slice(&[0xFF, px[0], px[1], px[2], px[3]]);
            }
        }
        prev = px;
    }
    out.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 1]);
    Ok(out)
}

// ---------------------------------------------------------------- ICO
/// Ein Bild für ICO (Pixel `0xAARRGGBB`).
pub struct IcoImage<'a> {
    pub width: usize,
    pub height: usize,
    pub pixels: &'a [u32],
}

/// Optionen für ICO: Zielgrößen (das Bild wird je Größe von außen skaliert übergeben).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IcoOptions {
    /// Kantenlängen, z. B. `[16, 24, 32, 48, 64, 128, 256]`.
    pub sizes: Vec<u32>,
    /// Bilder ab dieser Größe als PNG speichern (Vorgabe 256).
    pub png_from: u32,
}

impl Default for IcoOptions {
    fn default() -> Self {
        IcoOptions {
            sizes: vec![16, 24, 32, 48, 64, 128, 256],
            png_from: 256,
        }
    }
}

/// ICO mit mehreren Größen; Bilder ≥ `png_from` als PNG, sonst als 32-Bit-DIB mit AND-Maske.
pub fn write_ico(images: &[IcoImage], png_from: u32) -> Result<Vec<u8>> {
    if images.is_empty() || images.len() > 255 {
        return Err(ExportError::Encode(fp_i18n::t("err-export-d-ico-count")));
    }
    let mut blobs = Vec::new();
    for im in images {
        let (w, h) = (im.width, im.height);
        if w == 0 || h == 0 || w > 256 || h > 256 || im.pixels.len() < w * h {
            return Err(ExportError::BadSize(w, h));
        }
        if w as u32 >= png_from || h as u32 >= png_from {
            blobs.push(write_png_truecolor(
                im.pixels,
                w,
                h,
                true,
                &PngOptions {
                    dpi: None,
                    ..PngOptions::default()
                },
            )?);
            continue;
        }
        let mut d = Vec::new();
        d.extend_from_slice(&40u32.to_le_bytes());
        d.extend_from_slice(&(w as i32).to_le_bytes());
        d.extend_from_slice(&((h * 2) as i32).to_le_bytes());
        d.extend_from_slice(&1u16.to_le_bytes());
        d.extend_from_slice(&32u16.to_le_bytes());
        d.extend_from_slice(&[0u8; 24]);
        for y in (0..h).rev() {
            for &c in &im.pixels[y * w..(y + 1) * w] {
                d.extend_from_slice(&[b(c) as u8, g(c) as u8, r(c) as u8, a(c) as u8]);
            }
        }
        let mstride = w.div_ceil(32) * 4;
        for y in (0..h).rev() {
            let mut row = vec![0u8; mstride];
            for x in 0..w {
                if a(im.pixels[y * w + x]) == 0 {
                    row[x / 8] |= 0x80 >> (x % 8);
                }
            }
            d.extend_from_slice(&row);
        }
        blobs.push(d);
    }
    let mut out = vec![0, 0, 1, 0];
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut off = 6 + 16 * images.len();
    for (im, blob) in images.iter().zip(&blobs) {
        out.push(if im.width >= 256 { 0 } else { im.width as u8 });
        out.push(if im.height >= 256 { 0 } else { im.height as u8 });
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(blob.len() as u32).to_le_bytes());
        out.extend_from_slice(&(off as u32).to_le_bytes());
        off += blob.len();
    }
    for blob in blobs {
        out.extend_from_slice(&blob);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unpackbits(d: &[u8], n: usize) -> Vec<u8> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < d.len() && out.len() < n {
            let c = d[i] as i8;
            i += 1;
            if c >= 0 {
                out.extend_from_slice(&d[i..i + c as usize + 1]);
                i += c as usize + 1;
            } else if c != -128 {
                out.extend(std::iter::repeat_n(d[i], (1 - c as i32) as usize));
                i += 1;
            }
        }
        out
    }

    #[test]
    fn packbits_roundtrip() {
        for row in [
            vec![1u8, 1, 1, 2, 3, 4, 4, 5, 5, 5, 5, 5],
            vec![7; 300],
            (0..200).map(|i| (i % 7) as u8).collect(),
            vec![9],
        ] {
            assert_eq!(unpackbits(&packbits(&row), row.len()), row);
        }
    }

    #[test]
    fn bmp_resolution_and_header_like_original() {
        // re\27 E5 (regeln.py): 2,5406 statt 2,54, abgeschnittene dpi
        assert_eq!(
            [72.0, 96.0, 150.0, 300.0].map(bmp_pels_per_meter),
            [2834, 3779, 5904, 11808]
        );
        assert_eq!(bmp_pels_per_meter(150.9), 5904);
        assert_eq!(bmp_dpi_from_ppm(2834), Some(72));
        assert_eq!(bmp_dpi_from_ppm(3779), Some(96));
        assert_eq!(bmp_dpi_from_ppm(0), None);
        let px = vec![0x80FF0000u32, 0xFF00FF00, 0x00000000, 0xFF0000FF];
        let le32 = |b: &[u8], o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let le16 = |b: &[u8], o: usize| u16::from_le_bytes(b[o..o + 2].try_into().unwrap());
        // 32 Bit: 40-Byte-Kopf, BI_RGB, BGRA ohne Masken
        let bmp = write_bmp(
            &RasterData::Truecolor {
                width: 2,
                height: 2,
                pixels: &px,
                alpha: true,
            },
            300.0,
        )
        .unwrap();
        assert_eq!(le32(&bmp, 14), 40);
        assert_eq!(le16(&bmp, 28), 32);
        assert_eq!(le32(&bmp, 30), 0);
        assert_eq!(le32(&bmp, 10), 54);
        assert_eq!((le32(&bmp, 38), le32(&bmp, 42)), (11808, 11808));
        assert_eq!(bmp.len(), 54 + 16);
        // unterste Zeile zuerst: Pixel (0,1) = 0x00000000, (1,1) = 0xFF0000FF
        assert_eq!(&bmp[54..62], &[0, 0, 0, 0, 0xFF, 0, 0, 0xFF]);
        // 8 Bit: biClrUsed 256, biClrImportant = Farbanzahl
        let img = IndexedImage {
            width: 2,
            height: 1,
            palette: vec![0xFF000000, 0xFFFFFFFF, 0xFFFF0000],
            indices: vec![0, 2],
            ..Default::default()
        };
        let bmp = write_bmp(&RasterData::Indexed(&img), 72.0).unwrap();
        assert_eq!((le32(&bmp, 14), le16(&bmp, 28), le32(&bmp, 30)), (40, 8, 0));
        assert_eq!((le32(&bmp, 46), le32(&bmp, 50)), (256, 3));
        assert_eq!(le32(&bmp, 38), 2834);
    }

    #[test]
    fn simple_formats() {
        let px = vec![0xFFFF0000u32, 0x80FFFFFF, 0xFF000000, 0x00000000];
        let t = write_tiff(
            &RasterData::Truecolor {
                width: 2,
                height: 2,
                pixels: &px,
                alpha: true,
            },
            72.0,
            TiffCompression::PackBits,
            Some("x"),
        )
        .unwrap();
        assert_eq!(&t[..4], b"II*\0");
        let bmp = write_bmp(
            &RasterData::Truecolor {
                width: 2,
                height: 2,
                pixels: &px,
                alpha: false,
            },
            72.0,
        )
        .unwrap();
        assert_eq!(&bmp[..2], b"BM");
        assert_eq!(bmp.len(), 14 + 40 + 16);
        let q = write_qoi(&px, 2, 2, true).unwrap();
        assert_eq!(&q[..4], b"qoif");
        let img = IndexedImage {
            width: 9,
            height: 1,
            palette: vec![0xFF000000, 0xFFFFFFFF],
            indices: vec![1, 0, 1, 0, 1, 0, 1, 0, 1],
            ..Default::default()
        };
        let wb = write_wbmp(&img).unwrap();
        assert_eq!(wb, vec![0, 0, 9, 1, 0b1010_1010, 0b1000_0000]);
        let ico = write_ico(
            &[IcoImage {
                width: 2,
                height: 2,
                pixels: &px,
            }],
            256,
        )
        .unwrap();
        assert_eq!(&ico[..6], &[0, 0, 1, 0, 1, 0]);
    }
}
