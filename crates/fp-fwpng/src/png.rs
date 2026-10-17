//! PNG-Container auf Chunk-Ebene (eigene Implementierung) und Bilddaten über das `png`-Crate.
//!
//! Eine `.fw.png` ist ein gültiges PNG (RGBA8, ohne Interlace); die privaten Chunks stehen vor `IDAT`
//! (02 §2.1). CRC-Fehler werden gemeldet, aber nicht als Fehler behandelt (das Original prüft sie nicht).

use crate::error::{FwError, Result};
use fp_model::FlatImage;
use std::io::Cursor;

/// PNG-Signatur.
pub const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// Ein Chunk: Typ, Daten, Lage in der Datei, CRC-Prüfung.
#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    pub kind: [u8; 4],
    pub data: Vec<u8>,
    /// Byte-Offset des Längenfelds in der Datei (0 bei erzeugten Chunks).
    pub offset: usize,
    pub crc_ok: bool,
}

impl Chunk {
    pub fn new(kind: &[u8; 4], data: Vec<u8>) -> Chunk {
        Chunk {
            kind: *kind,
            data,
            offset: 0,
            crc_ok: true,
        }
    }
    /// Typ als Text.
    pub fn kind_str(&self) -> String {
        self.kind.iter().map(|&c| c as char).collect()
    }
    pub fn is(&self, kind: &[u8; 4]) -> bool {
        &self.kind == kind
    }
}

/// Liest alle Chunks bis einschließlich `IEND` (bzw. bis zum Dateiende).
pub fn read_chunks(data: &[u8]) -> Result<Vec<Chunk>> {
    if data.len() < 8 || data[..8] != PNG_SIGNATURE {
        return Err(FwError::NotPng);
    }
    let mut pos = 8usize;
    let mut out = Vec::new();
    while pos + 8 <= data.len() {
        let len =
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        let kind = [data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]];
        let body_start = pos + 8;
        let body_end = body_start
            .checked_add(len)
            .ok_or(FwError::TruncatedChunk { offset: pos })?;
        if body_end + 4 > data.len() {
            return Err(FwError::TruncatedChunk { offset: pos });
        }
        let body = &data[body_start..body_end];
        let crc = u32::from_be_bytes([
            data[body_end],
            data[body_end + 1],
            data[body_end + 2],
            data[body_end + 3],
        ]);
        let mut h = crc32fast::Hasher::new();
        h.update(&kind);
        h.update(body);
        let crc_ok = h.finalize() == crc;
        out.push(Chunk {
            kind,
            data: body.to_vec(),
            offset: pos,
            crc_ok,
        });
        pos = body_end + 4;
        if &kind == b"IEND" {
            break;
        }
    }
    Ok(out)
}

/// Schreibt Chunks hinter die PNG-Signatur.
pub fn write_chunks(chunks: &[Chunk]) -> Vec<u8> {
    let total: usize = 8 + chunks.iter().map(|c| c.data.len() + 12).sum::<usize>();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&PNG_SIGNATURE);
    for c in chunks {
        out.extend_from_slice(&(c.data.len() as u32).to_be_bytes());
        out.extend_from_slice(&c.kind);
        out.extend_from_slice(&c.data);
        let mut h = crc32fast::Hasher::new();
        h.update(&c.kind);
        h.update(&c.data);
        out.extend_from_slice(&h.finalize().to_be_bytes());
    }
    out
}

/// Inhalt von `IHDR`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ihdr {
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub color_type: u8,
    pub compression: u8,
    pub filter: u8,
    pub interlace: u8,
}

impl Ihdr {
    pub fn parse(data: &[u8]) -> Result<Ihdr> {
        if data.len() < 13 {
            return Err(FwError::BadChunk {
                kind: "IHDR".into(),
                msg: "zu kurz".into(),
            });
        }
        Ok(Ihdr {
            width: u32::from_be_bytes([data[0], data[1], data[2], data[3]]),
            height: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
            bit_depth: data[8],
            color_type: data[9],
            compression: data[10],
            filter: data[11],
            interlace: data[12],
        })
    }

    /// RGBA8 ohne Interlace (Standard des Originals).
    pub fn rgba8(width: u32, height: u32) -> Ihdr {
        Ihdr {
            width,
            height,
            bit_depth: 8,
            color_type: 6,
            compression: 0,
            filter: 0,
            interlace: 0,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(13);
        v.extend_from_slice(&self.width.to_be_bytes());
        v.extend_from_slice(&self.height.to_be_bytes());
        v.extend_from_slice(&[
            self.bit_depth,
            self.color_type,
            self.compression,
            self.filter,
            self.interlace,
        ]);
        v
    }
}

/// Dekodiert das flache Bild (alle `IDAT`) zu RGBA8. Benutzt das `png`-Crate; Paletten, Graustufen,
/// 16 Bit und Interlace werden nach RGBA8 umgerechnet.
pub fn decode_image(chunks: &[Chunk]) -> Result<FlatImage> {
    // Minimales PNG aus den Bild-relevanten Chunks zusammensetzen.
    let mut keep = Vec::new();
    let mut seen_ihdr = false;
    for c in chunks {
        match &c.kind {
            b"IHDR" => {
                seen_ihdr = true;
                keep.push(c.clone());
            }
            b"PLTE" | b"tRNS" | b"IDAT" | b"gAMA" | b"sBIT" => keep.push(c.clone()),
            _ => {}
        }
    }
    if !seen_ihdr {
        return Err(FwError::MissingIhdr);
    }
    keep.push(Chunk::new(b"IEND", Vec::new()));
    let bytes = write_chunks(&keep);
    let mut dec = png::Decoder::new(Cursor::new(bytes));
    dec.set_transformations(
        png::Transformations::EXPAND | png::Transformations::STRIP_16 | png::Transformations::ALPHA,
    );
    let mut reader = dec
        .read_info()
        .map_err(|e| FwError::PngDecode(e.to_string()))?;
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| FwError::PngDecode("Bild zu groß".into()))?;
    let mut buf = vec![0u8; size];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| FwError::PngDecode(e.to_string()))?;
    let (w, h) = (info.width, info.height);
    let n = w as usize * h as usize;
    let line = info.line_size;
    let rgba = match info.color_type {
        png::ColorType::Rgba => {
            let mut v = Vec::with_capacity(n * 4);
            for y in 0..h as usize {
                v.extend_from_slice(&buf[y * line..y * line + w as usize * 4]);
            }
            v
        }
        png::ColorType::GrayscaleAlpha => {
            let mut v = Vec::with_capacity(n * 4);
            for y in 0..h as usize {
                for x in 0..w as usize {
                    let g = buf[y * line + x * 2];
                    let a = buf[y * line + x * 2 + 1];
                    v.extend_from_slice(&[g, g, g, a]);
                }
            }
            v
        }
        png::ColorType::Rgb => {
            let mut v = Vec::with_capacity(n * 4);
            for y in 0..h as usize {
                for x in 0..w as usize {
                    let o = y * line + x * 3;
                    v.extend_from_slice(&[buf[o], buf[o + 1], buf[o + 2], 255]);
                }
            }
            v
        }
        png::ColorType::Grayscale => {
            let mut v = Vec::with_capacity(n * 4);
            for y in 0..h as usize {
                for x in 0..w as usize {
                    let g = buf[y * line + x];
                    v.extend_from_slice(&[g, g, g, 255]);
                }
            }
            v
        }
        png::ColorType::Indexed => {
            return Err(FwError::PngDecode("Palette nicht expandiert".into()));
        }
    };
    Ok(FlatImage {
        width: w,
        height: h,
        rgba,
    })
}

/// Zeilenfilter wie libpng (FP1-02): je Zeile der Filter mit der kleinsten Summe der Beträge der
/// gefilterten Bytes als vorzeichenbehaftete Bytes; bei Gleichstand gewinnt der erste in der
/// Reihenfolge None, Sub, Up, Average, Paeth. Das png-Paket wählt bei Gleichstand den späteren und
/// probiert None nicht (Zeile 0 Paeth statt Sub, Zeilen mit Up = Paeth Paeth statt Up).
pub fn filter_rows_libpng(data: &[u8], stride: usize, bpp: usize) -> Vec<u8> {
    let stride = stride.max(1);
    let mut out = Vec::with_capacity(data.len() + data.len() / stride + 1);
    let zero = vec![0u8; stride];
    let mut f = vec![0u8; stride];
    let mut best = vec![0u8; stride];
    let mut prev: &[u8] = &zero;
    for raw in data.chunks(stride) {
        let mut best_sum = u64::MAX;
        let mut best_ft = 0u8;
        for ft in 0..5u8 {
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
            let sum: u64 = f[..raw.len()]
                .iter()
                .map(|&v| (v as i8).unsigned_abs() as u64)
                .sum();
            if sum < best_sum {
                best_sum = sum;
                best_ft = ft;
                best[..raw.len()].copy_from_slice(&f[..raw.len()]);
            }
        }
        out.push(best_ft);
        out.extend_from_slice(&best[..raw.len()]);
        prev = raw;
    }
    out
}

/// Kodiert ein RGBA8-Bild und liefert nur die `IDAT`-Chunks (die übrigen Chunks setzt der Schreiber).
/// Zeilenfilter wie libpng ([`filter_rows_libpng`]), zlib Stufe 6.
pub fn encode_idat(img: &FlatImage) -> Result<Vec<Chunk>> {
    use std::io::Write;
    let (w, h) = (img.width.max(1) as usize, img.height.max(1) as usize);
    let expected = w * h * 4;
    let blank;
    let rgba: &[u8] = if img.rgba.len() == expected && img.width > 0 && img.height > 0 {
        &img.rgba
    } else {
        blank = vec![0u8; expected];
        &blank
    };
    let filtered = filter_rows_libpng(rgba, w * 4, 4);
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(6));
    e.write_all(&filtered)
        .map_err(|e| FwError::PngEncode(e.to_string()))?;
    let z = e.finish().map_err(|e| FwError::PngEncode(e.to_string()))?;
    Ok(vec![Chunk::new(b"IDAT", z)])
}

/// Eine PNG32-Datei (RGBA8, nicht vormultipliziert) für alle PNG-Schreibwege außerhalb von
/// `png_out` (Export über Brücke/MCP, `exportTo`, Skript-Export, Bildvorschau): `IHDR`, die
/// `extra`-Chunks (z. B. `sBIT`, `pHYs`), `IDAT` in Stücken zu 8192 Byte wie libpng, `IEND`.
/// Zeilenfilter wie libpng ([`filter_rows_libpng`]), zlib Stufe 6 (Q-37).
pub fn encode_rgba8(width: u32, height: u32, rgba: &[u8], extra: &[Chunk]) -> Result<Vec<u8>> {
    if width == 0 || height == 0 || rgba.len() != width as usize * height as usize * 4 {
        return Err(FwError::PngEncode(fp_i18n::t_args(
            "err-fwpng-rgba-size",
            &[
                ("width", fp_i18n::Value::from(width.to_string())),
                ("height", fp_i18n::Value::from(height.to_string())),
                ("bytes", fp_i18n::Value::from(rgba.len().to_string())),
            ],
        )));
    }
    let z: Vec<u8> = encode_idat(&FlatImage {
        width,
        height,
        rgba: rgba.to_vec(),
    })?
    .into_iter()
    .flat_map(|c| c.data)
    .collect();
    let mut chunks = vec![Chunk::new(b"IHDR", Ihdr::rgba8(width, height).to_bytes())];
    chunks.extend(extra.iter().cloned());
    chunks.extend(z.chunks(8192).map(|p| Chunk::new(b"IDAT", p.to_vec())));
    chunks.push(Chunk::new(b"IEND", Vec::new()));
    Ok(write_chunks(&chunks))
}

/// Liest den Schlüssel eines `tEXt`-Chunks und den Text (Latin-1).
pub fn parse_text(data: &[u8]) -> (String, String) {
    let (k, v) = match data.iter().position(|&b| b == 0) {
        Some(i) => (&data[..i], &data[i + 1..]),
        None => (data, &[][..]),
    };
    (latin1(k), latin1(v))
}

/// Erzeugt einen `tEXt`-Chunk (Latin-1; nicht darstellbare Zeichen werden zu `?`).
pub fn make_text(key: &str, text: &str) -> Chunk {
    let mut d = to_latin1(key);
    d.push(0);
    d.extend(to_latin1(text));
    Chunk::new(b"tEXt", d)
}

/// XMP aus `iTXt` lesen: (Schlüssel, Text). Komprimierte `iTXt` werden entpackt.
pub fn parse_itxt(data: &[u8]) -> Option<(String, String)> {
    let z = data.iter().position(|&b| b == 0)?;
    let key = latin1(&data[..z]);
    let rest = data.get(z + 1..)?;
    let (flag, _method) = (*rest.first()?, *rest.get(1)?);
    let rest = rest.get(2..)?;
    let l = rest.iter().position(|&b| b == 0)?;
    let rest = rest.get(l + 1..)?;
    let t = rest.iter().position(|&b| b == 0)?;
    let text = rest.get(t + 1..)?;
    let text = if flag == 1 {
        crate::zlib::inflate(text).ok()?
    } else {
        text.to_vec()
    };
    Some((key, String::from_utf8_lossy(&text).into_owned()))
}

/// Erzeugt einen unkomprimierten `iTXt`-Chunk.
pub fn make_itxt(key: &str, text: &str) -> Chunk {
    let mut d = to_latin1(key);
    d.extend_from_slice(&[0, 0, 0, 0, 0]);
    d.extend_from_slice(text.as_bytes());
    Chunk::new(b"iTXt", d)
}

fn latin1(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

fn to_latin1(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| if (c as u32) < 256 { c as u8 } else { b'?' })
        .collect()
}

/// `pHYs`: (Pixel je Einheit x, y, Einheit 1 = Meter).
pub fn parse_phys(data: &[u8]) -> Option<(u32, u32, u8)> {
    if data.len() < 9 {
        return None;
    }
    Some((
        u32::from_be_bytes([data[0], data[1], data[2], data[3]]),
        u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
        data[8],
    ))
}

/// Pixel je Meter für `pHYs` aus einer Auflösung in dpi wie im Original:
/// `floor(trunc(dpi) / 0,0254)` (BA-035). Das kleine Epsilon fängt Werte wie 254 dpi ab,
/// die rechnerisch knapp unter der Ganzzahl landen.
pub fn phys_ppm(dpi: f64) -> u32 {
    if !dpi.is_finite() || dpi <= 0.0 {
        return 0;
    }
    (dpi.trunc() / 0.0254 + 1e-6)
        .floor()
        .clamp(0.0, u32::MAX as f64) as u32
}

/// Pixel je Meter für `pHYs` aus einer Auflösung in Pixel/cm wie im Original:
/// `trunc(px/cm) · 100` (BA-124).
pub fn phys_ppm_per_cm(ppcm: f64) -> u32 {
    if !ppcm.is_finite() || ppcm <= 0.0 {
        return 0;
    }
    (ppcm.trunc() * 100.0).clamp(0.0, u32::MAX as f64) as u32
}

/// `pHYs`-Chunk aus Auflösung und Einheit (0 = Pixel/Zoll, 1 = Pixel/cm) wie im Original.
///
/// Regel des Originals (BA-035, gemessen): Auflösung erst auf ganze dpi abschneiden, dann
/// `floor(dpi / 0,0254)` – 72 dpi → 2834, 150,6 dpi → 5905, 299,7 dpi → 11771.
/// Einheit Pixel/cm (BA-124, gemessen): `trunc(px/cm) · 100` – 28,35 → 2800, 59,06 → 5900.
pub fn make_phys(resolution: f32, unit: i32) -> Chunk {
    let v = if unit == 1 {
        phys_ppm_per_cm(resolution as f64)
    } else {
        phys_ppm(resolution as f64)
    };
    let mut d = Vec::with_capacity(9);
    d.extend_from_slice(&v.to_be_bytes());
    d.extend_from_slice(&v.to_be_bytes());
    d.push(1);
    Chunk::new(b"pHYs", d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_roundtrip_and_image() {
        let img = FlatImage {
            width: 3,
            height: 2,
            rgba: (0..24).map(|i| (i * 10) as u8).collect(),
        };
        let mut chunks = vec![Chunk::new(b"IHDR", Ihdr::rgba8(3, 2).to_bytes())];
        chunks.extend(encode_idat(&img).unwrap());
        chunks.push(make_text("Software", "Firepixel"));
        chunks.push(Chunk::new(b"IEND", vec![]));
        let bytes = write_chunks(&chunks);
        let back = read_chunks(&bytes).unwrap();
        assert!(back.iter().all(|c| c.crc_ok));
        assert_eq!(back.len(), chunks.len());
        let dec = decode_image(&back).unwrap();
        assert_eq!(dec, img);
        assert_eq!(
            parse_text(&back[back.len() - 2].data),
            ("Software".into(), "Firepixel".into())
        );
    }

    #[test]
    fn truncated_is_error() {
        let mut b = PNG_SIGNATURE.to_vec();
        b.extend_from_slice(&[0, 0, 0, 50, b'I', b'H', b'D', b'R', 1, 2]);
        assert!(matches!(
            read_chunks(&b),
            Err(FwError::TruncatedChunk { .. })
        ));
        assert!(matches!(read_chunks(b"nope"), Err(FwError::NotPng)));
    }

    #[test]
    fn itxt_roundtrip() {
        let c = make_itxt("Description", "<x:xmpmeta/>");
        assert_eq!(
            parse_itxt(&c.data),
            Some(("Description".into(), "<x:xmpmeta/>".into()))
        );
    }

    /// FP1-02: Gleichstand wie libpng – Zeile 0 einer Fläche: Sub (= Paeth) gewinnt, nicht Paeth.
    fn filter_counts(img: &FlatImage) -> Vec<usize> {
        use std::io::Read;
        let z: Vec<u8> = encode_idat(img)
            .expect("IDAT")
            .into_iter()
            .flat_map(|c| c.data)
            .collect();
        let mut raw = Vec::new();
        flate2::read::ZlibDecoder::new(&z[..])
            .read_to_end(&mut raw)
            .expect("zlib");
        let stride = img.width as usize * 4 + 1;
        let f: Vec<u8> = (0..img.height as usize).map(|y| raw[y * stride]).collect();
        (0..5u8)
            .map(|k| f.iter().filter(|x| **x == k).count())
            .collect()
    }

    #[test]
    fn k1_ties_pick_first_filter_like_libpng() {
        // waagerechter Verlauf, alle Zeilen gleich: Zeile 0 Sub (Gleichstand mit Paeth), danach Up (= Paeth)
        let (w, h) = (16u32, 4u32);
        let rgba: Vec<u8> = (0..w * h)
            .flat_map(|i| [((i % w) * 9) as u8, 40, 200, 255])
            .collect();
        assert_eq!(
            filter_counts(&FlatImage {
                width: w,
                height: h,
                rgba
            }),
            vec![0, 1, 3, 0, 0]
        );
    }

    /// FP1-02 gegen das Original (Lauf 8, `export-results/01`, Pixel gleich): Filter 0/3/285/0/12; den
    /// Exportweg selbst prüft `fp-mcp` `integration::k1_filters_match_original_export` (Q-37).
    #[test]
    fn k1_filters_match_original_export() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../re/_testdateien/par/run_lauf8_2026-10-16/export-results/files/01_images_only_result_orig_par.png");
        let Ok(data) = std::fs::read(&p) else { return };
        let chunks = read_chunks(&data).expect("Chunks");
        let img = decode_image(&chunks).expect("Bild");
        assert_eq!(filter_counts(&img), vec![0, 3, 285, 0, 12]);
    }
}
