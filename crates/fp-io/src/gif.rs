//! GIF-Leser (12 §5.1; Referenz `re\import\gif.py`): Logical Screen, globale/lokale Paletten, Graphic
//! Control Extension (Verzögerung, Löschmethode, Transparenzindex), NETSCAPE2.0-/ANIMEXTS-Schleife,
//! LZW mit Zeilensprung. Beschädigte Ströme liefern Fehler bzw. werden wie im Original mit dem
//! dekodierten Anfang aufgefüllt.

use fp_geom::IRect;
use fp_raster::RgbaImage;

use crate::raster::{Animation, AnimationFrame};
use crate::{ImportError, Result, check_size};

/// Ein GIF-Einzelbild (Indizes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GifImage {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    pub indices: Vec<u8>,
    pub local_palette: Option<Vec<[u8; 3]>>,
    pub interlaced: bool,
    pub transparent: Option<u8>,
    /// Verzögerung in 1/100 s.
    pub delay: u16,
    pub disposal: u8,
    /// LZW-Strom vollständig (Endcode gefunden, Pixelzahl erreicht).
    pub complete: bool,
}

/// Dekodierte GIF-Datei.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GifFile {
    pub width: u16,
    pub height: u16,
    pub background_index: u8,
    pub global_palette: Option<Vec<[u8; 3]>>,
    pub frames: Vec<GifImage>,
    /// NETSCAPE-Schleife (`Some(0)` = endlos), `None` = Block fehlt.
    pub loop_count: Option<u16>,
    pub comments: Vec<Vec<u8>>,
}

/// LZW-Dekodierung (min. Codegröße 2…11). Liefert die Indizes (auf `npix` gekürzt/aufgefüllt) und
/// ob der Strom vollständig war.
pub fn lzw_decode(data: &[u8], min_code: u8, npix: usize) -> (Vec<u8>, bool) {
    let min_code = min_code.clamp(1, 11) as u32;
    let clear = 1usize << min_code;
    let eoi = clear + 1;
    let mut size = min_code + 1;
    // Wörterbuch als (Präfix, Endzeichen, Länge)
    let mut prefix = vec![0u16; 4096];
    let mut suffix = vec![0u8; 4096];
    let mut length = vec![0u16; 4096];
    for i in 0..clear.min(4096) {
        suffix[i] = i as u8;
        length[i] = 1;
    }
    let mut next = eoi + 1;
    let mut out: Vec<u8> = Vec::with_capacity(npix);
    let mut prev: Option<usize> = None;
    let (mut buf, mut bits, mut pos) = (0u64, 0u32, 0usize);
    let mut ended = false;
    let mut tmp: Vec<u8> = Vec::with_capacity(4096);
    loop {
        while bits < size && pos < data.len() {
            buf |= (data[pos] as u64) << bits;
            bits += 8;
            pos += 1;
        }
        if bits < size {
            break;
        }
        let code = (buf & ((1 << size) - 1)) as usize;
        buf >>= size;
        bits -= size;
        if code == clear {
            size = min_code + 1;
            next = eoi + 1;
            prev = None;
            continue;
        }
        if code == eoi {
            ended = true;
            break;
        }
        let emit_code =
            if (code < next && (code < clear || code > eoi)) || (code == next && prev.is_some()) {
                code
            } else {
                break;
            };
        // Zeichenkette von emit_code (bei code == next: prev + erstes Zeichen von prev)
        tmp.clear();
        let mut first_char;
        if code == next {
            let p = prev.unwrap_or(0);
            let mut c = p;
            loop {
                tmp.push(suffix[c]);
                if length[c] <= 1 {
                    break;
                }
                c = prefix[c] as usize;
            }
            tmp.reverse();
            first_char = tmp[0];
            tmp.push(first_char);
        } else {
            let mut c = emit_code;
            loop {
                tmp.push(suffix[c]);
                if length[c] <= 1 {
                    break;
                }
                c = prefix[c] as usize;
            }
            tmp.reverse();
            first_char = tmp[0];
        }
        if let Some(p) = prev
            && next < 4096
        {
            prefix[next] = p as u16;
            suffix[next] = first_char;
            length[next] = length[p].saturating_add(1);
            next += 1;
        }
        let _ = &mut first_char;
        out.extend_from_slice(&tmp);
        prev = Some(code);
        if next == (1 << size) && size < 12 {
            size += 1;
        }
        if out.len() >= npix && ended {
            break;
        }
        if out.len() > npix + 4096 {
            break;
        }
    }
    let complete = ended && out.len() >= npix;
    out.resize(npix, 0);
    (out, complete)
}

fn read_palette(b: &[u8], p: &mut usize, n: usize) -> Result<Vec<[u8; 3]>> {
    let bytes = b
        .get(*p..*p + 3 * n)
        .ok_or_else(|| ImportError::Corrupt(fp_i18n::t("err-import-gif-palette-cut")))?;
    *p += 3 * n;
    Ok(bytes
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| [c[0], c[1], c[2]])
        .collect())
}

/// GIF lesen.
pub fn decode(b: &[u8]) -> Result<GifFile> {
    if b.len() < 13 || &b[..3] != b"GIF" {
        return Err(ImportError::Corrupt(fp_i18n::t("err-import-not-gif")));
    }
    let u16le = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
    let (w, h, flags, bg) = (u16le(6), u16le(8), b[10], b[11]);
    let mut p = 13usize;
    let global_palette = if flags & 0x80 != 0 {
        Some(read_palette(b, &mut p, 2 << (flags & 7))?)
    } else {
        None
    };
    let mut g = GifFile {
        width: w,
        height: h,
        background_index: bg,
        global_palette,
        frames: Vec::new(),
        loop_count: None,
        comments: Vec::new(),
    };
    let mut gce: Option<(u8, u16, Option<u8>)> = None;
    while p < b.len() {
        let t = b[p];
        p += 1;
        match t {
            0x3B => break,
            0x21 => {
                let label = *b
                    .get(p)
                    .ok_or_else(|| ImportError::Corrupt(fp_i18n::t("err-import-gif-ext-cut")))?;
                p += 1;
                let mut blocks: Vec<&[u8]> = Vec::new();
                loop {
                    let n = *b.get(p).ok_or_else(|| {
                        ImportError::Corrupt(fp_i18n::t("err-import-gif-subblock-cut"))
                    })? as usize;
                    p += 1;
                    if n == 0 {
                        break;
                    }
                    blocks.push(b.get(p..p + n).ok_or_else(|| {
                        ImportError::Corrupt(fp_i18n::t("err-import-gif-subblock-cut"))
                    })?);
                    p += n;
                }
                match label {
                    0xF9 if blocks.first().is_some_and(|x| x.len() >= 4) => {
                        let x = blocks[0];
                        gce = Some((
                            (x[0] >> 2) & 7,
                            u16::from_le_bytes([x[1], x[2]]),
                            if x[0] & 1 != 0 { Some(x[3]) } else { None },
                        ));
                    }
                    0xFF if blocks.first().is_some_and(|x| {
                        x.starts_with(b"NETSCAPE2.0") || x.starts_with(b"ANIMEXTS1.0")
                    }) =>
                    {
                        if let Some(x) = blocks.get(1)
                            && x.len() >= 3
                            && x[0] == 1
                        {
                            g.loop_count = Some(u16::from_le_bytes([x[1], x[2]]));
                        }
                    }
                    0xFE => g.comments.push(blocks.concat()),
                    _ => {}
                }
            }
            0x2C => {
                let d = b
                    .get(p..p + 9)
                    .ok_or_else(|| ImportError::Corrupt(fp_i18n::t("err-import-image-desc-cut")))?;
                let (x, y, fw, fh, fl) = (
                    u16::from_le_bytes([d[0], d[1]]),
                    u16::from_le_bytes([d[2], d[3]]),
                    u16::from_le_bytes([d[4], d[5]]),
                    u16::from_le_bytes([d[6], d[7]]),
                    d[8],
                );
                p += 9;
                let local_palette = if fl & 0x80 != 0 {
                    Some(read_palette(b, &mut p, 2 << (fl & 7))?)
                } else {
                    None
                };
                let mc = *b.get(p).ok_or_else(|| {
                    ImportError::Corrupt(fp_i18n::t("err-import-lzw-size-missing"))
                })?;
                p += 1;
                let mut data = Vec::new();
                while let Some(&n) = b.get(p) {
                    p += 1;
                    if n == 0 {
                        break;
                    }
                    let n = n as usize;
                    let Some(chunk) = b.get(p..p + n) else {
                        data.extend_from_slice(&b[p.min(b.len())..]);
                        p = b.len();
                        break;
                    };
                    data.extend_from_slice(chunk);
                    p += n;
                }
                let npix = fw as usize * fh as usize;
                if npix as u64 > crate::MAX_PIXELS {
                    return Err(ImportError::TooLarge(fw as u32, fh as u32));
                }
                let (mut idx, complete) = lzw_decode(&data, mc, npix);
                let interlaced = fl & 0x40 != 0;
                if interlaced && fw > 0 {
                    let wu = fw as usize;
                    let hu = fh as usize;
                    let mut order: Vec<usize> = (0..hu).step_by(8).collect();
                    order.extend((4..hu).step_by(8));
                    order.extend((2..hu).step_by(4));
                    order.extend((1..hu).step_by(2));
                    let mut rows = vec![0u8; npix];
                    for (k, row) in order.into_iter().enumerate() {
                        rows[row * wu..(row + 1) * wu].copy_from_slice(&idx[k * wu..(k + 1) * wu]);
                    }
                    idx = rows;
                }
                let (disposal, delay, transparent) = gce.take().unwrap_or((0, 0, None));
                g.frames.push(GifImage {
                    x,
                    y,
                    w: fw,
                    h: fh,
                    indices: idx,
                    local_palette,
                    interlaced,
                    transparent,
                    delay,
                    disposal,
                    complete,
                });
            }
            _ => break,
        }
    }
    if g.frames.is_empty() {
        return Err(ImportError::Corrupt(fp_i18n::t("err-import-gif-no-image")));
    }
    Ok(g)
}

impl GifFile {
    /// Als Animation (Einzelbilder an ihrer Lage, transparente Pixel Alpha 0).
    pub fn into_animation(self) -> Animation {
        let has_t = self.frames.iter().any(|f| f.transparent.is_some());
        let mut frames = Vec::with_capacity(self.frames.len());
        for f in &self.frames {
            let pal = f.local_palette.as_ref().or(self.global_palette.as_ref());
            let mut rgba = Vec::with_capacity(f.indices.len() * 4);
            for &i in &f.indices {
                if Some(i) == f.transparent {
                    let c = pal
                        .and_then(|p| p.get(i as usize))
                        .copied()
                        .unwrap_or([0, 0, 0]);
                    rgba.extend_from_slice(&[c[0], c[1], c[2], 0]);
                } else {
                    let c = pal
                        .and_then(|p| p.get(i as usize))
                        .copied()
                        .unwrap_or([0, 0, 0]);
                    rgba.extend_from_slice(&[c[0], c[1], c[2], 255]);
                }
            }
            let r = IRect::new(
                f.x as i32,
                f.y as i32,
                f.x as i32 + f.w as i32,
                f.y as i32 + f.h as i32,
            );
            let image = RgbaImage::from_rgba_bytes(r, &rgba)
                .unwrap_or_else(|_| RgbaImage::empty_at(f.x as i32, f.y as i32));
            frames.push(AnimationFrame {
                image,
                x: f.x as i32,
                y: f.y as i32,
                delay_cs: f.delay as u32,
                disposal: f.disposal,
                replace: false,
            });
        }
        let loop_count = match self.loop_count {
            None => -1,
            Some(n) => n as i32,
        };
        let mut w = self.width as u32;
        let mut h = self.height as u32;
        if check_size(w, h).is_err() {
            w = frames
                .iter()
                .map(|f| (f.x + f.image.width() as i32).max(1) as u32)
                .max()
                .unwrap_or(1);
            h = frames
                .iter()
                .map(|f| (f.y + f.image.height() as i32).max(1) as u32)
                .max()
                .unwrap_or(1);
        }
        Animation {
            width: w,
            height: h,
            frames,
            loop_count,
            has_transparency: has_t,
            palette: self.global_palette.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_with_writer_bytes() {
        // GIF aus fw_gif.write_gif: 2×2, Palette weiß/rot, Indizes 0 1 1 0
        let g = [
            71u8, 73, 70, 56, 57, 97, 2, 0, 2, 0, 128, 0, 0, 255, 255, 255, 255, 0, 0, 33, 249, 4,
            4, 7, 0, 255, 0, 44, 0, 0, 0, 0, 2, 0, 2, 0, 0, 2, 3, 68, 2, 5, 0, 59,
        ];
        let d = decode(&g).unwrap();
        assert_eq!(d.frames[0].indices, vec![0, 1, 1, 0]);
        assert_eq!(d.frames[0].delay, 7);
        assert_eq!(d.frames[0].disposal, 1);
        assert!(d.frames[0].complete);
        let a = d.into_animation();
        assert_eq!(a.loop_count, -1);
        assert_eq!(
            a.frames[0].image.pixel(1, 0),
            fp_color::Rgba8::new(255, 0, 0, 255)
        );
        assert!(decode(b"GIF89a").is_err());
        assert!(decode(&g[..30]).is_err());
    }
}
