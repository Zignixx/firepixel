//! GIF89a-Schreiber im Original-Aufbau (08 §5.2, `@00403af0`, LZW `@00403710`) und verlustbehaftetes
//! GIF (`@0074b520`). Referenz `re\export\fw_gif.py` (LZW-Ströme der Installation 126/126 byte-gleich).
//!
//! Aufbau: immer `GIF89a`; Palettenbits = kleinstes `b ≥ 1` mit `2^b ≥ n`, Tabelle mit Schwarz
//! aufgefüllt; optional NETSCAPE2.0-Schleife (nur Animation, Schleife ≥ 0) und Kommentar; je Bild eine
//! GCE (Löschmethode, Verzögerung, Transparenzindex sonst `0xFF`); lokale Palette nur, wenn sie von der
//! globalen abweicht; LZW mit Mindestcodegröße `max(2, b)`, Breitenwechsel nach der Ausgabe,
//! Neustart bei 4096 Codes, 255er-Unterblöcke.

use std::collections::HashMap;

/// Ein Bild eines (animierten) GIF.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct GifFrame {
    /// Indizes zeilenweise (`w·h`).
    pub indices: Vec<u8>,
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    /// Verzögerung in 1/100 s.
    pub delay: u16,
    /// Löschmethode 0…3 (GIF89a-Codes).
    pub disposal: u8,
    /// Transparenzindex.
    pub transparent: Option<u8>,
    /// Lokale Palette (RGB), nur geschrieben, wenn sie von der globalen abweicht.
    pub palette: Option<Vec<[u8; 3]>>,
}

/// LZW-Kodierung wie das Original (gieriges Standard-LZW, IJG-`wrgif`-Breitenwechsel).
pub fn lzw_encode(indices: &[u8], min_code_size: u8) -> Vec<u8> {
    let min_code_size = min_code_size.clamp(2, 8) as u32;
    let clear = 1u32 << min_code_size;
    let eoi = clear + 1;
    let mut n_bits = min_code_size + 1;
    let mut maxcode = (1u32 << n_bits) - 1;
    let mut free = clear + 2;
    let mut acc: u64 = 0;
    let mut nacc = 0u32;
    let mut out = Vec::with_capacity(indices.len() / 2 + 16);
    let mut table: HashMap<(u32, u8), u32> = HashMap::new();

    let mut emit =
        |code: u32, n_bits: &mut u32, maxcode: &mut u32, free: u32, out: &mut Vec<u8>| {
            acc |= (code as u64) << nacc;
            nacc += *n_bits;
            while nacc >= 8 {
                out.push((acc & 0xFF) as u8);
                acc >>= 8;
                nacc -= 8;
            }
            if free > *maxcode {
                *n_bits += 1;
                *maxcode = if *n_bits == 12 {
                    4096
                } else {
                    (1 << *n_bits) - 1
                };
            }
        };

    emit(clear, &mut n_bits, &mut maxcode, free, &mut out);
    let mut it = indices.iter();
    let Some(&first) = it.next() else {
        emit(eoi, &mut n_bits, &mut maxcode, free, &mut out);
        if nacc > 0 {
            out.push((acc & 0xFF) as u8);
        }
        return out;
    };
    let mut waiting = first as u32;
    for &c in it {
        if let Some(&code) = table.get(&(waiting, c)) {
            waiting = code;
            continue;
        }
        emit(waiting, &mut n_bits, &mut maxcode, free, &mut out);
        if free < 4096 {
            table.insert((waiting, c), free);
            free += 1;
        } else {
            table.clear();
            free = clear + 2;
            emit(clear, &mut n_bits, &mut maxcode, free, &mut out);
            n_bits = min_code_size + 1;
            maxcode = (1 << n_bits) - 1;
        }
        waiting = c as u32;
    }
    emit(waiting, &mut n_bits, &mut maxcode, free, &mut out);
    emit(eoi, &mut n_bits, &mut maxcode, free, &mut out);
    if nacc > 0 {
        out.push((acc & 0xFF) as u8);
    }
    out
}

/// Datenbytes in 255er-Unterblöcke mit Endblock 0.
pub fn sub_blocks(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 255 + 2);
    for ch in data.chunks(255) {
        out.push(ch.len() as u8);
        out.extend_from_slice(ch);
    }
    out.push(0);
    out
}

/// Palettenbits: kleinstes `b ≥ 1` mit `2^b ≥ n`.
pub fn palette_bits(ncolors: usize) -> u8 {
    let mut b = 1u8;
    while (1usize << b) < ncolors && b < 8 {
        b += 1;
    }
    b
}

/// Zeilenreihenfolge beim Zeilensprung (8/8, 4/8, 2/4, 1/2).
pub fn interlace_order(h: usize) -> Vec<usize> {
    let mut v: Vec<usize> = (0..h).step_by(8).collect();
    v.extend((4..h).step_by(8));
    v.extend((2..h).step_by(4));
    v.extend((1..h).step_by(2));
    v
}

/// Optionen des GIF-Schreibers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GifOptions {
    /// Schleife (`FLPi`): `Some(n)` mit n ≥ 0 schreibt den NETSCAPE2.0-Block (nur bei mehreren Bildern).
    pub loop_count: Option<u16>,
    /// Kommentar (≤ 250 Zeichen, Latin-1).
    pub comment: Option<String>,
    /// Zeilensprung (`GFIb`).
    pub interlaced: bool,
}

/// Schreibt ein GIF89a im Original-Aufbau.
pub fn write_gif(
    width: u16,
    height: u16,
    palette: &[[u8; 3]],
    frames: &[GifFrame],
    opts: &GifOptions,
) -> Vec<u8> {
    let n = palette.len();
    let bits = palette_bits(n);
    let size = 1usize << bits;
    let mut out = Vec::new();
    out.extend_from_slice(b"GIF89a");
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.push(0x80 | ((bits - 1) << 4) | (bits - 1));
    out.push(0);
    out.push(0);
    for k in 0..size {
        out.extend_from_slice(palette.get(k).unwrap_or(&[0, 0, 0]));
    }
    if let Some(lp) = opts.loop_count
        && frames.len() > 1
    {
        out.extend_from_slice(b"\x21\xff\x0bNETSCAPE2.0\x03\x01");
        out.extend_from_slice(&lp.to_le_bytes());
        out.push(0);
    }
    if let Some(c) = &opts.comment
        && !c.is_empty()
    {
        let bytes: Vec<u8> = c
            .chars()
            .map(|ch| if (ch as u32) < 256 { ch as u8 } else { b'?' })
            .take(250)
            .collect();
        out.extend_from_slice(&[0x21, 0xFE, bytes.len() as u8]);
        out.extend_from_slice(&bytes);
        out.push(0);
    }
    for fr in frames {
        let pk = ((fr.disposal & 7) << 2) | u8::from(fr.transparent.is_some());
        out.extend_from_slice(&[0x21, 0xF9, 0x04, pk]);
        out.extend_from_slice(&fr.delay.to_le_bytes());
        out.push(fr.transparent.unwrap_or(0xFF));
        out.push(0);
        let mut flags = if opts.interlaced { 0x40u8 } else { 0 };
        let mut fbits = bits;
        let local = fr.palette.as_ref().filter(|lp| lp.as_slice() != palette);
        if let Some(lp) = local {
            fbits = palette_bits(lp.len());
            flags |= 0x80 | (fbits - 1);
        }
        out.push(0x2C);
        for v in [fr.x, fr.y, fr.w, fr.h] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.push(flags);
        if let Some(lp) = local {
            for k in 0..(1usize << fbits) {
                out.extend_from_slice(lp.get(k).unwrap_or(&[0, 0, 0]));
            }
        }
        let mcs = fbits.max(2);
        let (w, h) = (fr.w as usize, fr.h as usize);
        let mut idx: Vec<u8> = fr.indices.clone();
        idx.resize(w * h, 0);
        if opts.interlaced && w > 0 {
            let mut v = Vec::with_capacity(idx.len());
            for row in interlace_order(h) {
                v.extend_from_slice(&idx[row * w..(row + 1) * w]);
            }
            idx = v;
        }
        out.push(mcs);
        out.extend(sub_blocks(&lzw_encode(&idx, mcs)));
    }
    out.push(0x3B);
    out
}

/// Geschätzte Größe des LZW-Stroms (Bytes inkl. Unterblöcke), für Moduswahl und Größenschätzung.
pub fn lzw_size(indices: &[u8], min_code_size: u8) -> usize {
    let n = lzw_encode(indices, min_code_size).len();
    n + n.div_ceil(255) + 1
}

/// Verlustbehaftetes GIF (`@0074b520`): simuliert LZW über das Indexbild und ersetzt ein Pixel durch
/// die Farbe eines vorhandenen Wörterbuch-Nachfolgers, wenn dessen Abstand² < `lossy²` ist (außer am
/// Ende einer gleichfarbigen Strecke ≥ 3 mit Abstand² ≥ `(lossy − 4)²`). Abstand zwischen
/// Palettenfarben (A).
pub fn lossy_simulate(index_img: &[u8], palette: &[[u8; 3]], lossy: i32) -> Vec<u8> {
    let n = palette.len();
    let (mut b, mut s) = (2u32, 4usize);
    while s < n {
        b += 1;
        s *= 2;
    }
    let first = (1u32 << b) + 2;
    let mut out = index_img.to_vec();
    let mut entries: HashMap<u32, (u8, u32)> = HashMap::new();
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut nxt = first;
    let mut cur: Option<u32> = None;
    let lim = (lossy as i64) * (lossy as i64);
    let d2 = |i: u8, j: u8| -> i64 {
        let (x, y) = (
            palette.get(i as usize).copied().unwrap_or([0; 3]),
            palette.get(j as usize).copied().unwrap_or([0; 3]),
        );
        (0..3).map(|k| (x[k] as i64 - y[k] as i64).pow(2)).sum()
    };
    let color_of = |code: u32, entries: &HashMap<u32, (u8, u32)>| -> u8 {
        if code < first {
            code as u8
        } else {
            entries.get(&code).map(|e| e.0).unwrap_or(0)
        }
    };
    for px in out.iter_mut() {
        let idx = *px;
        let Some(c0) = cur else {
            cur = Some(idx as u32);
            continue;
        };
        let mut best = lim + 1;
        let mut bestc: Option<u32> = None;
        let mut exact: Option<u32> = None;
        if let Some(ch) = children.get(&c0) {
            for &code in ch {
                let ci = color_of(code, &entries);
                if ci == idx {
                    exact = Some(code);
                    break;
                }
                let dd = d2(ci, idx);
                if dd < best {
                    best = dd;
                    bestc = Some(code);
                }
            }
        }
        if let Some(e) = exact {
            cur = Some(e);
            continue;
        }
        if let Some(bc) = bestc
            && best < lim
        {
            let mut allsame = true;
            let mut run = 1;
            let mut c = c0;
            while c > first {
                let Some(&(ci, pre)) = entries.get(&c) else {
                    break;
                };
                if ci != idx {
                    allsame = false;
                    break;
                }
                run += 1;
                c = pre;
            }
            let l4 = (lossy as i64 - 4).pow(2);
            if !(c == idx as u32 && run >= 3 && allsame && best >= l4) {
                *px = color_of(bc, &entries);
                cur = Some(bc);
                continue;
            }
        }
        if nxt < 4096 {
            entries.insert(nxt, (idx, c0));
            children.entry(c0).or_default().push(nxt);
            nxt += 1;
        } else {
            entries.clear();
            children.clear();
            nxt = first;
        }
        cur = Some(idx as u32);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lzw_small() {
        // Werte aus fw_gif.lzw_encode
        assert_eq!(lzw_encode(&[0, 0, 0, 0], 2), vec![132, 81]);
        let pat: Vec<u8> = [0u8, 1, 2, 3, 0, 1, 2, 3, 3, 3, 3, 3, 1, 1, 1, 1].repeat(20);
        assert_eq!(
            lzw_encode(&pat, 2)[..20],
            [
                68, 52, 134, 195, 19, 31, 84, 58, 141, 193, 72, 101, 189, 82, 55, 158, 81, 27, 212,
                137
            ]
        );
        assert_eq!(palette_bits(3), 2);
        assert_eq!(palette_bits(2), 1);
        assert_eq!(palette_bits(256), 8);
        assert_eq!(interlace_order(5), vec![0, 4, 2, 1, 3]);
    }

    #[test]
    fn gif_structure() {
        let f = GifFrame {
            indices: vec![0, 1, 1, 0],
            w: 2,
            h: 2,
            delay: 7,
            disposal: 1,
            ..Default::default()
        };
        let g = write_gif(
            2,
            2,
            &[[255, 255, 255], [255, 0, 0]],
            std::slice::from_ref(&f),
            &GifOptions::default(),
        );
        assert_eq!(
            g,
            vec![
                71, 73, 70, 56, 57, 97, 2, 0, 2, 0, 128, 0, 0, 255, 255, 255, 255, 0, 0, 33, 249,
                4, 4, 7, 0, 255, 0, 44, 0, 0, 0, 0, 2, 0, 2, 0, 0, 2, 3, 68, 2, 5, 0, 59
            ]
        );
        let lossy = lossy_simulate(&[0, 1, 0, 1, 0, 1], &[[0, 0, 0], [2, 2, 2]], 10);
        assert_eq!(lossy.len(), 6);
    }
}
