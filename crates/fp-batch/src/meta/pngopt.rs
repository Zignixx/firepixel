//! Verlustfreie PNG-Optimierung (Schritt `optimize.png`, SPEC-BATCH §3.12). Eigene Umsetzung statt
//! `oxipng`: oxipng 9 bindet `libdeflater` (C) fest ein und ist damit nicht reines Rust (Bericht
//! BATCH-5). Verfahren wie dort: Farbtyp und Bittiefe verlustfrei verkleinern (Palette ≤ 256
//! Farben mit `tRNS`, Graustufen 1/2/4/8 Bit, Grau+Alpha, Alphakanal weg bei deckenden Bildern),
//! optional Farbe unter vollständig transparenten Pixeln auf 0 setzen, mehrere Filterstrategien
//! (0–4, adaptiv; ab Stufe 4 zeilenweise nach kleinster Kompression), Deflate Stufe 9; das
//! kleinste Ergebnis gewinnt, nie größer als die Eingabe.

use std::collections::HashMap;
use std::io::Write;

/// Pixel einer PNG-Datei als RGBA8 (Palette/tRNS/Grau aufgelöst, 16 Bit → 8 Bit).
pub fn decode_rgba(png: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(png));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let n = (w as usize).checked_mul(h as usize)?;
    let src = &buf[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => src.to_vec(),
        png::ColorType::Rgb => src
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => src
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => src.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    (rgba.len() == n * 4).then_some((w, h, rgba))
}

/// Optionen der Optimierung.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PngOptOptions {
    /// 1 (schnell) … 6 (gründlich).
    pub level: u8,
    /// Farbe unter Alpha 0 auf 0 setzen.
    pub alpha_clean: bool,
}

impl Default for PngOptOptions {
    fn default() -> Self {
        PngOptOptions {
            level: 2,
            alpha_clean: true,
        }
    }
}

/// Kandidat: Farbtyp, Bittiefe, Zeilen ungefiltert (gepackt), Zusatz-Chunks (PLTE/tRNS).
struct Candidate {
    color_type: u8,
    depth: u8,
    bpp: usize,
    rows: Vec<Vec<u8>>,
    plte: Option<Vec<u8>>,
    trns: Option<Vec<u8>>,
}

fn pack(values: &[u8], depth: u8) -> Vec<u8> {
    if depth == 8 {
        return values.to_vec();
    }
    let per = (8 / depth) as usize;
    let mut out = vec![0u8; values.len().div_ceil(per)];
    for (i, &v) in values.iter().enumerate() {
        let shift = 8 - depth as usize * (i % per + 1);
        out[i / per] |= v << shift;
    }
    out
}

fn depth_for(n: usize) -> u8 {
    match n {
        0..=2 => 1,
        3..=4 => 2,
        5..=16 => 4,
        _ => 8,
    }
}

fn candidates(w: usize, h: usize, px: &[[u8; 4]]) -> Vec<Candidate> {
    let opaque = px.iter().all(|p| p[3] == 255);
    let gray = px.iter().all(|p| p[0] == p[1] && p[1] == p[2]);
    let mut out = Vec::new();
    // Palette
    let mut index: HashMap<[u8; 4], u8> = HashMap::new();
    let mut pal: Vec<[u8; 4]> = Vec::new();
    let mut fits = true;
    for p in px {
        if !index.contains_key(p) {
            if pal.len() == 256 {
                fits = false;
                break;
            }
            index.insert(*p, pal.len() as u8);
            pal.push(*p);
        }
    }
    if fits && !pal.is_empty() {
        // transparente Einträge zuerst (kurzes tRNS), dann nach Häufigkeit
        let mut count: HashMap<[u8; 4], u64> = HashMap::new();
        for p in px {
            *count.entry(*p).or_default() += 1;
        }
        pal.sort_by(|a, b| {
            (a[3] == 255)
                .cmp(&(b[3] == 255))
                .then(count[b].cmp(&count[a]))
                .then(a.cmp(b))
        });
        let index: HashMap<[u8; 4], u8> =
            pal.iter().enumerate().map(|(i, p)| (*p, i as u8)).collect();
        let depth = depth_for(pal.len());
        let rows = px
            .chunks(w)
            .map(|r| pack(&r.iter().map(|p| index[p]).collect::<Vec<_>>(), depth))
            .collect();
        let plte = pal.iter().flat_map(|p| [p[0], p[1], p[2]]).collect();
        let nt = pal.iter().take_while(|p| p[3] < 255).count();
        let trns = (nt > 0).then(|| pal[..nt].iter().map(|p| p[3]).collect());
        out.push(Candidate {
            color_type: 3,
            depth,
            bpp: 1,
            rows,
            plte: Some(plte),
            trns,
        });
    }
    if gray && opaque {
        // kleinste Bittiefe, in der alle Grauwerte exakt darstellbar sind
        let depth = [1u8, 2, 4, 8].into_iter().find(|&d| {
            let m = (1u32 << d) - 1;
            px.iter().all(|p| (p[0] as u32 * m).is_multiple_of(255))
        });
        let d = depth.unwrap_or(8);
        let m = (1u32 << d) - 1;
        let rows = px
            .chunks(w)
            .map(|r| {
                pack(
                    &r.iter()
                        .map(|p| (p[0] as u32 * m / 255) as u8)
                        .collect::<Vec<_>>(),
                    d,
                )
            })
            .collect();
        out.push(Candidate {
            color_type: 0,
            depth: d,
            bpp: 1,
            rows,
            plte: None,
            trns: None,
        });
    } else if gray {
        let rows = px
            .chunks(w)
            .map(|r| r.iter().flat_map(|p| [p[0], p[3]]).collect())
            .collect();
        out.push(Candidate {
            color_type: 4,
            depth: 8,
            bpp: 2,
            rows,
            plte: None,
            trns: None,
        });
    }
    if opaque {
        let rows = px
            .chunks(w)
            .map(|r| r.iter().flat_map(|p| [p[0], p[1], p[2]]).collect())
            .collect();
        out.push(Candidate {
            color_type: 2,
            depth: 8,
            bpp: 3,
            rows,
            plte: None,
            trns: None,
        });
    } else {
        let rows = px
            .chunks(w)
            .map(|r| r.iter().flat_map(|p| *p).collect())
            .collect();
        out.push(Candidate {
            color_type: 6,
            depth: 8,
            bpp: 4,
            rows,
            plte: None,
            trns: None,
        });
    }
    let _ = h;
    out
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let (ia, ib, ic) = (a as i16, b as i16, c as i16);
    let p = ia + ib - ic;
    let (pa, pb, pc) = ((p - ia).abs(), (p - ib).abs(), (p - ic).abs());
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

fn filter_row(ft: u8, row: &[u8], prev: &[u8], bpp: usize, out: &mut Vec<u8>) {
    out.clear();
    out.push(ft);
    for i in 0..row.len() {
        let a = if i >= bpp { row[i - bpp] } else { 0 };
        let b = prev.get(i).copied().unwrap_or(0);
        let c = if i >= bpp {
            prev.get(i - bpp).copied().unwrap_or(0)
        } else {
            0
        };
        let x = row[i];
        out.push(match ft {
            0 => x,
            1 => x.wrapping_sub(a),
            2 => x.wrapping_sub(b),
            3 => x.wrapping_sub(((a as u16 + b as u16) / 2) as u8),
            _ => x.wrapping_sub(paeth(a, b, c)),
        });
    }
}

fn zlib(data: &[u8], level: u32) -> Vec<u8> {
    let mut e = flate2::write::ZlibEncoder::new(
        Vec::with_capacity(data.len() / 2),
        flate2::Compression::new(level),
    );
    let _ = e.write_all(data);
    e.finish().unwrap_or_default()
}

/// Strategie: fester Filter 0–4, 5 = adaptiv (kleinste Summe der Beträge), 6 = zeilenweise nach
/// kleinster Kompression.
fn filtered(c: &Candidate, strategy: u8) -> Vec<u8> {
    let mut out = Vec::new();
    let empty = Vec::new();
    let mut tmp = Vec::new();
    let mut best = Vec::new();
    for (y, row) in c.rows.iter().enumerate() {
        let prev = if y == 0 { &empty } else { &c.rows[y - 1] };
        match strategy {
            0..=4 => {
                filter_row(strategy, row, prev, c.bpp, &mut tmp);
                out.extend_from_slice(&tmp);
            }
            5 => {
                let mut best_sum = u64::MAX;
                for ft in 0..5 {
                    filter_row(ft, row, prev, c.bpp, &mut tmp);
                    let sum: u64 = tmp[1..]
                        .iter()
                        .map(|&v| (v as i8).unsigned_abs() as u64)
                        .sum();
                    if sum < best_sum {
                        best_sum = sum;
                        std::mem::swap(&mut best, &mut tmp);
                    }
                }
                out.extend_from_slice(&best);
            }
            _ => {
                let mut best_len = usize::MAX;
                for ft in 0..5 {
                    filter_row(ft, row, prev, c.bpp, &mut tmp);
                    let len = zlib(&tmp, 6).len();
                    if len < best_len {
                        best_len = len;
                        std::mem::swap(&mut best, &mut tmp);
                    }
                }
                out.extend_from_slice(&best);
            }
        }
    }
    out
}

/// Kodiert RGBA-Pixel als möglichst kleines PNG (ohne Metadaten-Chunks).
pub fn encode_rgba(w: u32, h: u32, rgba: &[u8], opts: PngOptOptions) -> Option<Vec<u8>> {
    let n = (w as usize).checked_mul(h as usize)?;
    if w == 0 || h == 0 || rgba.len() != n * 4 {
        return None;
    }
    let mut px: Vec<[u8; 4]> = rgba.as_chunks::<4>().0.to_vec();
    if opts.alpha_clean {
        for p in &mut px {
            if p[3] == 0 {
                *p = [0, 0, 0, 0];
            }
        }
    }
    let level = opts.level.clamp(1, 6);
    let (strategies, zl): (&[u8], u32) = match level {
        1 => (&[0, 5], 6),
        2 | 3 => (&[0, 1, 2, 3, 4, 5], 9),
        _ => (&[0, 1, 2, 3, 4, 5, 6], 9),
    };
    let mut best: Option<(Vec<u8>, &Candidate)> = None;
    let cands = candidates(w as usize, h as usize, &px);
    for c in &cands {
        // Paletten-/Grau-Bilder mit < 8 Bit profitieren nicht von Filtern
        let strat: &[u8] = if c.depth < 8 || c.color_type == 3 {
            &[0, 5]
        } else {
            strategies
        };
        for &s in strat {
            let z = zlib(&filtered(c, s), zl);
            if best.as_ref().is_none_or(|(b, _)| z.len() < b.len()) {
                best = Some((z, c));
            }
        }
    }
    let (idat, c) = best?;
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[c.depth, c.color_type, 0, 0, 0]);
    let mut chunks = vec![(*b"IHDR", ihdr)];
    if let Some(p) = &c.plte {
        chunks.push((*b"PLTE", p.clone()));
    }
    if let Some(t) = &c.trns {
        chunks.push((*b"tRNS", t.clone()));
    }
    chunks.push((*b"IDAT", idat));
    chunks.push((*b"IEND", Vec::new()));
    Some(super::container::png_assemble(&chunks))
}

/// Optimiert eine PNG-Datei verlustfrei; liefert das Original, wenn es nicht kleiner wird oder die
/// Datei nicht lesbar ist (z. B. APNG – dort bleibt es beim Kodierer). Metadaten-Chunks fallen weg
/// (sie werden danach nach dem Schritt „Metadaten“ neu eingefügt).
pub fn optimize(png: &[u8], opts: PngOptOptions) -> Vec<u8> {
    let is_apng =
        super::container::png_chunks(png).is_some_and(|c| c.iter().any(|(t, _)| t == b"acTL"));
    if is_apng {
        return png.to_vec();
    }
    match decode_rgba(png).and_then(|(w, h, rgba)| encode_rgba(w, h, &rgba, opts)) {
        Some(o) if o.len() < png.len() => o,
        _ => png.to_vec(),
    }
}
