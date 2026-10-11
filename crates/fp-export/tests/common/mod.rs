//! Hilfen für die Testdatei-Vergleiche: Pfade nach `..\re`, PNG- und GIF-Leser (nur für Tests).
#![allow(dead_code)]

use std::path::PathBuf;

/// `C:\…\REA\re` relativ zum Crate.
pub fn re_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("re")
}

/// PNG → (Breite, Höhe, Pixel `0xAARRGGBB`).
pub fn read_png(path: &std::path::Path) -> Option<(usize, usize, Vec<u32>)> {
    let f = std::fs::File::open(path).ok()?;
    let mut dec = png::Decoder::new(std::io::BufReader::new(f));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut r = dec.read_info().ok()?;
    let mut buf = vec![0; r.output_buffer_size()?];
    let info = r.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width as usize, info.height as usize);
    let ch = info.color_type.samples();
    let mut px = Vec::with_capacity(w * h);
    for i in 0..w * h {
        let p = &buf[i * ch..i * ch + ch];
        let (r, g, b, a) = match ch {
            1 => (p[0], p[0], p[0], 255),
            2 => (p[0], p[0], p[0], p[1]),
            3 => (p[0], p[1], p[2], 255),
            _ => (p[0], p[1], p[2], p[3]),
        };
        px.push(((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | b as u32);
    }
    Some((w, h, px))
}

/// PNG8 roh: (Breite, Höhe, Palette RGBA, Indizes).
pub fn read_png8(path: &std::path::Path) -> Option<(usize, usize, Vec<u32>, Vec<u8>)> {
    let f = std::fs::File::open(path).ok()?;
    let mut dec = png::Decoder::new(std::io::BufReader::new(f));
    dec.set_transformations(png::Transformations::IDENTITY);
    let mut r = dec.read_info().ok()?;
    let (plte, trns) = {
        let i = r.info();
        (i.palette.as_ref()?.to_vec(), i.trns.as_ref().map(|t| t.to_vec()).unwrap_or_default())
    };
    let mut buf = vec![0; r.output_buffer_size()?];
    let info = r.next_frame(&mut buf).ok()?;
    if info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    let pal = plte
        .chunks(3)
        .enumerate()
        .map(|(i, c)| ((trns.get(i).copied().unwrap_or(255) as u32) << 24) | ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32)
        .collect();
    let (w, h) = (info.width as usize, info.height as usize);
    let mut idx = Vec::with_capacity(w * h);
    for y in 0..h {
        idx.extend_from_slice(&buf[y * info.line_size..y * info.line_size + w]);
    }
    Some((w, h, pal, idx))
}

#[derive(Debug, Default, Clone)]
pub struct GifFrame {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub lct: Option<Vec<[u8; 3]>>,
    pub indices: Vec<u8>,
    pub transparent: Option<u8>,
    pub delay: u16,
    pub disposal: u8,
    /// LZW-Rohdaten (ohne Unterblock-Längen), Mindestcodegröße, Zeilensprung.
    pub lzw: Vec<u8>,
    pub min_code: u8,
    pub interlaced: bool,
}

#[derive(Debug, Default, Clone)]
pub struct Gif {
    pub width: usize,
    pub height: usize,
    pub gct: Vec<[u8; 3]>,
    pub frames: Vec<GifFrame>,
    pub loop_count: Option<u16>,
}

fn lzw_decode(data: &[u8], min_code: u32, npix: usize) -> Vec<u8> {
    let clear = 1u32 << min_code;
    let eoi = clear + 1;
    let mut size = min_code + 1;
    let mut dict: Vec<Vec<u8>> = (0..clear).map(|i| vec![i as u8]).collect();
    dict.push(vec![]);
    dict.push(vec![]);
    let mut out = Vec::new();
    let mut prev: Option<Vec<u8>> = None;
    let (mut buf, mut bits, mut pos) = (0u64, 0u32, 0usize);
    loop {
        while bits < size && pos < data.len() {
            buf |= (data[pos] as u64) << bits;
            bits += 8;
            pos += 1;
        }
        if bits < size {
            break;
        }
        let code = (buf & ((1 << size) - 1)) as u32;
        buf >>= size;
        bits -= size;
        if code == clear {
            size = min_code + 1;
            dict.truncate(clear as usize + 2);
            prev = None;
            continue;
        }
        if code == eoi {
            break;
        }
        let entry = if (code as usize) < dict.len() {
            let e = dict[code as usize].clone();
            if let Some(p) = &prev
                && dict.len() < 4096
            {
                let mut n = p.clone();
                n.push(e[0]);
                dict.push(n);
            }
            e
        } else if let Some(p) = &prev {
            let mut e = p.clone();
            e.push(p[0]);
            if dict.len() < 4096 {
                dict.push(e.clone());
            }
            e
        } else {
            break;
        };
        out.extend_from_slice(&entry);
        prev = Some(entry);
        if dict.len() == (1 << size) && size < 12 {
            size += 1;
        }
        if out.len() >= npix {
            break;
        }
    }
    out.resize(npix, 0);
    out
}

/// Minimaler GIF-Leser.
pub fn read_gif(path: &std::path::Path) -> Option<Gif> {
    let b = std::fs::read(path).ok()?;
    if b.get(..3)? != b"GIF" {
        return None;
    }
    let u16le = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
    let mut g = Gif { width: u16le(6) as usize, height: u16le(8) as usize, ..Default::default() };
    let flags = b[10];
    let mut p = 13;
    if flags & 0x80 != 0 {
        let n = 2 << (flags & 7);
        g.gct = b[p..p + 3 * n].chunks(3).map(|c| [c[0], c[1], c[2]]).collect();
        p += 3 * n;
    }
    let mut gce: Option<(u8, u16, Option<u8>)> = None;
    while p < b.len() {
        let t = b[p];
        p += 1;
        match t {
            0x3B => break,
            0x21 => {
                let label = b[p];
                p += 1;
                let mut blocks = Vec::new();
                loop {
                    let n = b[p] as usize;
                    p += 1;
                    if n == 0 {
                        break;
                    }
                    blocks.push(b[p..p + n].to_vec());
                    p += n;
                }
                if label == 0xF9 && !blocks.is_empty() && blocks[0].len() >= 4 {
                    let f = blocks[0][0];
                    gce = Some(((f >> 2) & 7, u16::from_le_bytes([blocks[0][1], blocks[0][2]]), if f & 1 != 0 { Some(blocks[0][3]) } else { None }));
                } else if label == 0xFF && blocks.len() > 1 && blocks[0].starts_with(b"NETSCAPE2.0") && blocks[1].len() >= 3 {
                    g.loop_count = Some(u16::from_le_bytes([blocks[1][1], blocks[1][2]]));
                }
            }
            0x2C => {
                let (x, y, w, h) = (u16le(p) as usize, u16le(p + 2) as usize, u16le(p + 4) as usize, u16le(p + 6) as usize);
                let fl = b[p + 8];
                p += 9;
                let mut lct = None;
                if fl & 0x80 != 0 {
                    let n = 2 << (fl & 7);
                    lct = Some(b[p..p + 3 * n].chunks(3).map(|c| [c[0], c[1], c[2]]).collect());
                    p += 3 * n;
                }
                let mc = b[p] as u32;
                p += 1;
                let mut data = Vec::new();
                loop {
                    let n = b[p] as usize;
                    p += 1;
                    if n == 0 {
                        break;
                    }
                    data.extend_from_slice(&b[p..p + n]);
                    p += n;
                }
                let mut idx = lzw_decode(&data, mc, w * h);
                if fl & 0x40 != 0 && w > 0 {
                    let mut order: Vec<usize> = (0..h).step_by(8).collect();
                    order.extend((4..h).step_by(8));
                    order.extend((2..h).step_by(4));
                    order.extend((1..h).step_by(2));
                    let mut rows = vec![0u8; w * h];
                    for (k, row) in order.into_iter().enumerate() {
                        rows[row * w..(row + 1) * w].copy_from_slice(&idx[k * w..(k + 1) * w]);
                    }
                    idx = rows;
                }
                let (disposal, delay, transparent) = gce.take().unwrap_or((0, 0, None));
                g.frames.push(GifFrame { x, y, w, h, lct, indices: idx, transparent, delay, disposal, lzw: data, min_code: mc as u8, interlaced: fl & 0x40 != 0 });
            }
            _ => return None,
        }
    }
    Some(g)
}
