//! JPEG-Kodierer wie das Original (08 §4): IJG-Farbumrechnung und -Unterabtastung, Glättung
//! (`smoothing_factor = 16·s`), islow-FDCT, **Original-Quantisierung mit Viertel-Schritt-Rundung**
//! (`q = (|c| + Schritt/4) div Schritt`), selektives JPEG (blockweise Faktoren), eigene Qualitätstabellen
//! ([`crate::jpeg_tables`]), optimierte Huffman-Tabellen, Baseline oder progressiv.
//!
//! Progressiv wie IJG `jpeg_simple_progression` (10 Scans mit Spektralauswahl und schrittweiser
//! Annäherung, optimierte Tabellen je Scan); Byte-Gleichheit zum Original ungeprüft (A). Jeder
//! Standard-Dekoder liest das Ergebnis.

use crate::jpeg_tables::{FwJpegTables, ZIGZAG};
use crate::px;
use crate::{ExportError, Result};

/// Parameter des Kodierers.
#[derive(Clone, Debug, PartialEq)]
pub struct JpegParams {
    /// Qualität 1…100.
    pub quality: i32,
    /// Glättung 0…7.
    pub smoothing: i32,
    /// `jpegSubsampling` (0 = 4:4:4; sonst 4:2:0, solange Qualität ≤ 85).
    pub subsampling: i32,
    pub progressive: bool,
    /// Optimierte Huffman-Tabellen (`optimize_coding = optimized`).
    pub optimize: bool,
    /// Auflösung (JFIF-Dichte in dpi).
    pub dpi: f32,
    /// Selektives JPEG: (Qualität der Maske, Maske `w·h`, ≠ 0 = ausgewählt).
    pub selective: Option<(i32, Vec<u8>)>,
    /// Feste Quantisierungstabellen (Luminanz, Chrominanz; natürliche Reihenfolge) statt der aus
    /// `quality` abgeleiteten (PDF-Qualitätsstufen, re 28 E6).
    pub fixed_tables: Option<Box<([u16; 64], [u16; 64])>>,
}

impl Default for JpegParams {
    fn default() -> Self {
        JpegParams {
            quality: 80,
            smoothing: 0,
            subsampling: 0,
            progressive: false,
            optimize: true,
            dpi: 72.0,
            selective: None,
            fixed_tables: None,
        }
    }
}

// ---------------------------------------------------------------- FDCT (islow)
const CONST_BITS: i32 = 13;
const PASS1_BITS: i32 = 2;
const FIX_0_298631336: i32 = 2446;
const FIX_0_390180644: i32 = 3196;
const FIX_0_541196100: i32 = 4433;
const FIX_0_765366865: i32 = 6270;
const FIX_0_899976223: i32 = 7373;
const FIX_1_175875602: i32 = 9633;
const FIX_1_501321110: i32 = 12299;
const FIX_1_847759065: i32 = 15137;
const FIX_1_961570560: i32 = 16069;
const FIX_2_053119869: i32 = 16819;
const FIX_2_562915447: i32 = 20995;
const FIX_3_072711026: i32 = 25172;

#[inline]
fn descale(x: i32, n: i32) -> i32 {
    (x + (1 << (n - 1))) >> n
}

/// Ganzzahlige DCT wie IJG `jpeg_fdct_islow`; Eingabe um 128 zentriert, Ausgabe mit Faktor 8.
pub fn fdct_islow(data: &mut [i32; 64]) {
    for row in 0..8 {
        let d = &mut data[row * 8..row * 8 + 8];
        let tmp0 = d[0] + d[7];
        let tmp7 = d[0] - d[7];
        let tmp1 = d[1] + d[6];
        let tmp6 = d[1] - d[6];
        let tmp2 = d[2] + d[5];
        let tmp5 = d[2] - d[5];
        let tmp3 = d[3] + d[4];
        let tmp4 = d[3] - d[4];
        let tmp10 = tmp0 + tmp3;
        let tmp13 = tmp0 - tmp3;
        let tmp11 = tmp1 + tmp2;
        let tmp12 = tmp1 - tmp2;
        d[0] = (tmp10 + tmp11) << PASS1_BITS;
        d[4] = (tmp10 - tmp11) << PASS1_BITS;
        let z1 = (tmp12 + tmp13) * FIX_0_541196100;
        d[2] = descale(z1 + tmp13 * FIX_0_765366865, CONST_BITS - PASS1_BITS);
        d[6] = descale(z1 + tmp12 * -FIX_1_847759065, CONST_BITS - PASS1_BITS);
        let (z1, z2, z3, z4) = (tmp4 + tmp7, tmp5 + tmp6, tmp4 + tmp6, tmp5 + tmp7);
        let z5 = (z3 + z4) * FIX_1_175875602;
        let (t4, t5, t6, t7) = (
            tmp4 * FIX_0_298631336,
            tmp5 * FIX_2_053119869,
            tmp6 * FIX_3_072711026,
            tmp7 * FIX_1_501321110,
        );
        let (z1, z2) = (z1 * -FIX_0_899976223, z2 * -FIX_2_562915447);
        let z3 = z3 * -FIX_1_961570560 + z5;
        let z4 = z4 * -FIX_0_390180644 + z5;
        d[7] = descale(t4 + z1 + z3, CONST_BITS - PASS1_BITS);
        d[5] = descale(t5 + z2 + z4, CONST_BITS - PASS1_BITS);
        d[3] = descale(t6 + z2 + z3, CONST_BITS - PASS1_BITS);
        d[1] = descale(t7 + z1 + z4, CONST_BITS - PASS1_BITS);
    }
    for col in 0..8 {
        let g = |i: usize| data[i * 8 + col];
        let tmp0 = g(0) + g(7);
        let tmp7 = g(0) - g(7);
        let tmp1 = g(1) + g(6);
        let tmp6 = g(1) - g(6);
        let tmp2 = g(2) + g(5);
        let tmp5 = g(2) - g(5);
        let tmp3 = g(3) + g(4);
        let tmp4 = g(3) - g(4);
        let tmp10 = tmp0 + tmp3;
        let tmp13 = tmp0 - tmp3;
        let tmp11 = tmp1 + tmp2;
        let tmp12 = tmp1 - tmp2;
        let mut o = [0i32; 8];
        o[0] = descale(tmp10 + tmp11, PASS1_BITS);
        o[4] = descale(tmp10 - tmp11, PASS1_BITS);
        let z1 = (tmp12 + tmp13) * FIX_0_541196100;
        o[2] = descale(z1 + tmp13 * FIX_0_765366865, CONST_BITS + PASS1_BITS);
        o[6] = descale(z1 + tmp12 * -FIX_1_847759065, CONST_BITS + PASS1_BITS);
        let (z1, z2, z3, z4) = (tmp4 + tmp7, tmp5 + tmp6, tmp4 + tmp6, tmp5 + tmp7);
        let z5 = (z3 + z4) * FIX_1_175875602;
        let (t4, t5, t6, t7) = (
            tmp4 * FIX_0_298631336,
            tmp5 * FIX_2_053119869,
            tmp6 * FIX_3_072711026,
            tmp7 * FIX_1_501321110,
        );
        let (z1, z2) = (z1 * -FIX_0_899976223, z2 * -FIX_2_562915447);
        let z3 = z3 * -FIX_1_961570560 + z5;
        let z4 = z4 * -FIX_0_390180644 + z5;
        o[7] = descale(t4 + z1 + z3, CONST_BITS + PASS1_BITS);
        o[5] = descale(t5 + z2 + z4, CONST_BITS + PASS1_BITS);
        o[3] = descale(t6 + z2 + z3, CONST_BITS + PASS1_BITS);
        o[1] = descale(t7 + z1 + z4, CONST_BITS + PASS1_BITS);
        for (i, v) in o.into_iter().enumerate() {
            data[i * 8 + col] = v;
        }
    }
}

/// Original-Quantisierung eines Blocks (natürliche Reihenfolge, DCT-Ausgabe ×8). `factor` = `None`
/// für volle Qualität, sonst `(m, f)` des selektiven JPEG. Ergebnis in Einheiten von `Q`.
pub fn quantize_block(
    coef: &[i32; 64],
    q_natural: &[u16; 64],
    factor: Option<(i64, i64)>,
) -> [i32; 64] {
    let (lo, hi, cut) = match factor {
        None => (1i64, 1i64, 64i64),
        Some((m, f)) => (m.max(1), m.max(1) + 1, 64 - f),
    };
    let mut out = [0i32; 64];
    for k in 0..64 {
        let mut m = if (k as i64) <= cut { lo } else { hi };
        if k == 0 {
            m = (m + 1) / 2;
        }
        let step = (m * q_natural[k] as i64 * 8).max(1);
        let c = coef[k] as i64;
        let v = ((step >> 2) + c.abs()) / step;
        out[k] = ((if c < 0 { -v } else { v }) * m) as i32;
    }
    out
}

// ---------------------------------------------------------------- Farbe und Unterabtastung
fn fix(x: f64) -> i32 {
    (x * 65536.0 + 0.5) as i32
}

/// RGB → YCbCr wie IJG `jccolor.c` (16 Bit Festkomma).
pub fn rgb_to_ycc(r: i32, g: i32, b: i32) -> (u8, u8, u8) {
    const HALF: i32 = 1 << 15;
    const OFF: i32 = 128 << 16;
    let y = (fix(0.29900) * r + fix(0.58700) * g + fix(0.11400) * b + HALF) >> 16;
    let cb = (-fix(0.16874) * r - fix(0.33126) * g + fix(0.5) * b + OFF + HALF - 1) >> 16;
    let cr = (fix(0.5) * r - fix(0.41869) * g - fix(0.08131) * b + OFF + HALF - 1) >> 16;
    (
        y.clamp(0, 255) as u8,
        cb.clamp(0, 255) as u8,
        cr.clamp(0, 255) as u8,
    )
}

/// Eine Komponente als Ebene mit Breite/Höhe (auf Blockgrenzen erweitert).
#[derive(Clone)]
struct Plane {
    w: usize,
    h: usize,
    data: Vec<i32>,
}

impl Plane {
    fn at(&self, x: isize, y: isize) -> i32 {
        let x = x.clamp(0, self.w as isize - 1) as usize;
        let y = y.clamp(0, self.h as isize - 1) as usize;
        self.data[y * self.w + x]
    }
}

/// 1:1 mit optionaler Glättung (IJG `fullsize_smooth_downsample`).
fn fullsize(p: &Plane, sf: i32) -> Plane {
    if sf == 0 {
        return p.clone();
    }
    let memberscale = 65536 - sf * 512;
    let neighscale = sf * 64;
    let mut out = vec![0i32; p.w * p.h];
    for y in 0..p.h as isize {
        let col = |x: isize| p.at(x, y - 1) + p.at(x, y + 1) + p.at(x, y);
        for x in 0..p.w as isize {
            let member = p.at(x, y);
            let cur = col(x);
            let neigh = col(x - 1) + (cur - member) + col(x + 1);
            out[y as usize * p.w + x as usize] =
                (member * memberscale + neigh * neighscale + 32768) >> 16;
        }
    }
    Plane {
        w: p.w,
        h: p.h,
        data: out,
    }
}

/// 2:1 in beiden Richtungen (IJG `h2v2_downsample` bzw. `h2v2_smooth_downsample`).
fn h2v2(p: &Plane, sf: i32) -> Plane {
    let (ow, oh) = (p.w.div_ceil(2), p.h.div_ceil(2));
    let mut out = vec![0i32; ow * oh];
    for oy in 0..oh {
        let (y0, y1) = ((2 * oy) as isize, (2 * oy + 1) as isize);
        for ox in 0..ow {
            let (x0, x1) = ((2 * ox) as isize, (2 * ox + 1) as isize);
            let member = p.at(x0, y0) + p.at(x1, y0) + p.at(x0, y1) + p.at(x1, y1);
            let v = if sf == 0 {
                let bias = if ox % 2 == 0 { 1 } else { 2 };
                (member + bias) >> 2
            } else {
                let memberscale = 16384 - sf * 80;
                let neighscale = sf * 16;
                let (am, bm) = (y0 - 1, y1 + 1);
                let mut neigh = p.at(x0, am)
                    + p.at(x1, am)
                    + p.at(x0, bm)
                    + p.at(x1, bm)
                    + p.at(x0 - 1, y0)
                    + p.at(x1 + 1, y0)
                    + p.at(x0 - 1, y1)
                    + p.at(x1 + 1, y1);
                neigh += neigh;
                neigh += p.at(x0 - 1, am) + p.at(x1 + 1, am) + p.at(x0 - 1, bm) + p.at(x1 + 1, bm);
                (member * memberscale + neigh * neighscale + 32768) >> 16
            };
            out[oy * ow + ox] = v;
        }
    }
    Plane {
        w: ow,
        h: oh,
        data: out,
    }
}

// ---------------------------------------------------------------- Huffman
#[derive(Clone)]
struct Huff {
    bits: [u8; 17],
    vals: Vec<u8>,
    code: [u16; 256],
    size: [u8; 256],
}

impl Huff {
    fn new(bits: [u8; 17], vals: Vec<u8>) -> Huff {
        let mut h = Huff {
            bits,
            vals,
            code: [0; 256],
            size: [0; 256],
        };
        let mut code = 0u32;
        let mut k = 0usize;
        for l in 1..=16usize {
            for _ in 0..h.bits[l] {
                if let Some(&v) = h.vals.get(k) {
                    h.code[v as usize] = code as u16;
                    h.size[v as usize] = l as u8;
                }
                code += 1;
                k += 1;
            }
            code <<= 1;
        }
        h
    }

    /// Optimale Tabelle aus Häufigkeiten (IJG `jpeg_gen_optimal_table`, Längen ≤ 16).
    fn optimal(freq_in: &[u64; 256]) -> Huff {
        let mut freq = [0i64; 257];
        for i in 0..256 {
            freq[i] = freq_in[i] as i64;
        }
        freq[256] = 1;
        let mut codesize = [0i32; 257];
        let mut others = [-1i32; 257];
        loop {
            let mut c1 = -1i32;
            let mut v = i64::MAX;
            for (i, &f) in freq.iter().enumerate().take(257) {
                if f > 0 && f <= v {
                    v = f;
                    c1 = i as i32;
                }
            }
            let mut c2 = -1i32;
            v = i64::MAX;
            for (i, &f) in freq.iter().enumerate().take(257) {
                if f > 0 && f <= v && i as i32 != c1 {
                    v = f;
                    c2 = i as i32;
                }
            }
            if c2 < 0 {
                break;
            }
            let (c1u, c2u) = (c1 as usize, c2 as usize);
            freq[c1u] += freq[c2u];
            freq[c2u] = 0;
            let mut c = c1u;
            codesize[c] += 1;
            while others[c] >= 0 {
                c = others[c] as usize;
                codesize[c] += 1;
            }
            others[c] = c2;
            let mut c = c2u;
            codesize[c] += 1;
            while others[c] >= 0 {
                c = others[c] as usize;
                codesize[c] += 1;
            }
        }
        let mut bits = [0i32; 33];
        for &cs in codesize.iter() {
            if cs > 0 {
                bits[(cs as usize).min(32)] += 1;
            }
        }
        let mut i = 32;
        while i > 16 {
            while bits[i] > 0 {
                let mut j = i - 2;
                while bits[j] == 0 {
                    j -= 1;
                }
                bits[i] -= 2;
                bits[i - 1] += 1;
                bits[j + 1] += 2;
                bits[j] -= 1;
            }
            i -= 1;
        }
        while bits[i] == 0 {
            i -= 1;
        }
        bits[i] -= 1;
        let mut b17 = [0u8; 17];
        for l in 1..=16 {
            b17[l] = bits[l].max(0) as u8;
        }
        let mut vals = Vec::new();
        for l in 1..=32 {
            for (s, &cs) in codesize.iter().enumerate().take(256) {
                if cs == l {
                    vals.push(s as u8);
                }
            }
        }
        Huff::new(b17, vals)
    }
}

fn std_dc_lum() -> Huff {
    Huff::new(
        [0, 0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
        (0..12).collect(),
    )
}
fn std_dc_chrom() -> Huff {
    Huff::new(
        [0, 0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0],
        (0..12).collect(),
    )
}
fn std_ac_lum() -> Huff {
    let vals = vec![
        0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61,
        0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xa1, 0x08, 0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52,
        0xd1, 0xf0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x25,
        0x26, 0x27, 0x28, 0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45,
        0x46, 0x47, 0x48, 0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64,
        0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x83,
        0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99,
        0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6,
        0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3,
        0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8,
        0xe9, 0xea, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa,
    ];
    Huff::new([0, 0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 0x7d], vals)
}
fn std_ac_chrom() -> Huff {
    let vals = vec![
        0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61,
        0x71, 0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91, 0xa1, 0xb1, 0xc1, 0x09, 0x23, 0x33,
        0x52, 0xf0, 0x15, 0x62, 0x72, 0xd1, 0x0a, 0x16, 0x24, 0x34, 0xe1, 0x25, 0xf1, 0x17, 0x18,
        0x19, 0x1a, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44,
        0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63,
        0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a,
        0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97,
        0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4,
        0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca,
        0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7,
        0xe8, 0xe9, 0xea, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa,
    ];
    Huff::new([0, 0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 0x77], vals)
}

struct BitWriter {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl BitWriter {
    fn new() -> BitWriter {
        BitWriter {
            out: Vec::new(),
            acc: 0,
            n: 0,
        }
    }
    fn put(&mut self, code: u32, size: u32) {
        if size == 0 {
            return;
        }
        self.acc = (self.acc << size) | (code & ((1u32 << size) - 1));
        self.n += size;
        while self.n >= 8 {
            let b = ((self.acc >> (self.n - 8)) & 0xFF) as u8;
            self.out.push(b);
            if b == 0xFF {
                self.out.push(0);
            }
            self.n -= 8;
        }
        self.acc &= (1u32 << self.n).wrapping_sub(1);
    }
    fn flush(&mut self) {
        if self.n > 0 {
            let pad = 8 - self.n;
            self.put((1 << pad) - 1, pad);
        }
    }
}

fn nbits(v: i32) -> u32 {
    32 - v.unsigned_abs().leading_zeros()
}

/// Symbolquelle für Statistik (Häufigkeiten) oder Ausgabe.
trait Sink {
    fn sym(&mut self, table: usize, s: u8);
    fn raw(&mut self, v: u32, n: u32);
}

struct Stats([[u64; 256]; 8]);
impl Sink for Stats {
    fn sym(&mut self, t: usize, s: u8) {
        self.0[t][s as usize] += 1;
    }
    fn raw(&mut self, _: u32, _: u32) {}
}

struct Emit<'a> {
    bw: &'a mut BitWriter,
    tables: &'a [Huff],
}
impl Sink for Emit<'_> {
    fn sym(&mut self, t: usize, s: u8) {
        let h = &self.tables[t];
        self.bw
            .put(h.code[s as usize] as u32, h.size[s as usize] as u32);
    }
    fn raw(&mut self, v: u32, n: u32) {
        self.bw.put(v, n);
    }
}

fn magnitude(v: i32) -> (u32, u32) {
    let n = nbits(v);
    let bits = if v < 0 { (v - 1) as u32 } else { v as u32 };
    (n, bits & ((1u32 << n).wrapping_sub(1)))
}

/// Ein sequentieller Block: DC-Differenz + AC mit Lauflängen (`blk` in natürlicher Reihenfolge).
fn encode_block<S: Sink>(s: &mut S, blk: &[i32; 64], pred: &mut i32, dc_t: usize, ac_t: usize) {
    let diff = blk[0] - *pred;
    *pred = blk[0];
    let (n, bits) = magnitude(diff);
    s.sym(dc_t, n as u8);
    s.raw(bits, n);
    let mut run = 0u32;
    for &zi in ZIGZAG.iter().skip(1) {
        let v = blk[zi];
        if v == 0 {
            run += 1;
            continue;
        }
        while run > 15 {
            s.sym(ac_t, 0xF0);
            run -= 16;
        }
        let (n, bits) = magnitude(v);
        s.sym(ac_t, ((run << 4) | n) as u8);
        s.raw(bits, n);
        run = 0;
    }
    if run > 0 {
        s.sym(ac_t, 0x00);
    }
}

/// Komponente mit Blöcken (natürliche Reihenfolge, quantisiert).
struct Comp {
    id: u8,
    h: usize,
    v: usize,
    tq: usize,
    bw: usize,
    bh: usize,
    /// Blöcke je Zeile/Spalte bei nicht verschachtelten Scans (ohne MCU-Auffüllung).
    nbw: usize,
    nbh: usize,
    blocks: Vec<[i32; 64]>,
}

impl Comp {
    fn block(&self, bx: usize, by: usize) -> &[i32; 64] {
        &self.blocks[by.min(self.bh - 1) * self.bw + bx.min(self.bw - 1)]
    }
}

fn marker(out: &mut Vec<u8>, m: u8, data: &[u8]) {
    out.extend_from_slice(&[0xFF, m]);
    out.extend_from_slice(&((data.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(data);
}

fn dht(out: &mut Vec<u8>, class_id: u8, h: &Huff) {
    let mut d = vec![class_id];
    d.extend_from_slice(&h.bits[1..17]);
    d.extend_from_slice(&h.vals);
    marker(out, 0xC4, &d);
}

/// Kodiert ein Bild (Pixel `0xAARRGGBB`; nicht deckende Pixel werden über Weiß gemischt).
pub fn encode_jpeg(
    pixels: &[u32],
    w: usize,
    h: usize,
    params: &JpegParams,
    tables: &FwJpegTables,
) -> Result<Vec<u8>> {
    if w == 0 || h == 0 || w > 65535 || h > 65535 || pixels.len() < w * h {
        return Err(ExportError::BadSize(w, h));
    }
    let q = params.quality.clamp(1, 100);
    let sel = params.selective.as_ref().filter(|(_, m)| m.len() >= w * h);
    let sel_par = sel.and_then(|(qs, _)| tables.selective_params(q, *qs));
    let q_tab = sel_par.map(|p| p.0).unwrap_or(q);
    let mut qt = tables.qtables(q_tab);
    if let Some(f) = &params.fixed_tables {
        for k in 0..64 {
            qt.lum[k] = f.0[ZIGZAG[k]];
            qt.chrom[k] = f.1[ZIGZAG[k]];
        }
    }
    let mut lum_nat = [0u16; 64];
    let mut chrom_nat = [0u16; 64];
    for k in 0..64 {
        lum_nat[ZIGZAG[k]] = qt.lum[k];
        chrom_nat[ZIGZAG[k]] = qt.chrom[k];
    }
    let sub = !(params.subsampling == 0 || q > 85);
    let sf = params.smoothing.clamp(0, 7) * 16;
    // Farbumrechnung (Alpha über Weiß)
    let (mut yp, mut cbp, mut crp) = (
        Vec::with_capacity(w * h),
        Vec::with_capacity(w * h),
        Vec::with_capacity(w * h),
    );
    for &c in &pixels[..w * h] {
        let c = px::over_white(c);
        let (y, cb, cr) = rgb_to_ycc(px::r(c) as i32, px::g(c) as i32, px::b(c) as i32);
        yp.push(y as i32);
        cbp.push(cb as i32);
        crp.push(cr as i32);
    }
    let yplane = fullsize(&Plane { w, h, data: yp }, sf);
    let cplanes = [Plane { w, h, data: cbp }, Plane { w, h, data: crp }];
    let cplanes: Vec<Plane> = cplanes
        .iter()
        .map(|p| if sub { h2v2(p, sf) } else { fullsize(p, sf) })
        .collect();
    // Maske je Block: (maskiert ⇒ volle Qualität)
    let mask_full = |x0: usize, y0: usize, scale: usize| -> bool {
        let Some((qs, m)) = sel else { return true };
        let Some(_) = sel_par else { return true };
        let mut any = false;
        'o: for y in y0 * scale..((y0 + 8) * scale).min(h) {
            for x in x0 * scale..((x0 + 8) * scale).min(w) {
                if m[y * w + x] != 0 {
                    any = true;
                    break 'o;
                }
            }
        }
        if q < *qs { any } else { !any }
    };
    let (mcu_w, mcu_h) = if sub { (16, 16) } else { (8, 8) };
    let (mcux, mcuy) = (w.div_ceil(mcu_w), h.div_ceil(mcu_h));
    let make = |p: &Plane, id: u8, hv: usize, tq: usize, qn: &[u16; 64], scale: usize| -> Comp {
        let (bw, bh) = (mcux * hv, mcuy * hv);
        let mut blocks = Vec::with_capacity(bw * bh);
        for by in 0..bh {
            for bx in 0..bw {
                let mut d = [0i32; 64];
                for yy in 0..8 {
                    for xx in 0..8 {
                        d[yy * 8 + xx] = p.at((bx * 8 + xx) as isize, (by * 8 + yy) as isize) - 128;
                    }
                }
                fdct_islow(&mut d);
                let factor = if mask_full(bx * 8, by * 8, scale) {
                    None
                } else {
                    sel_par.map(|(_, m, f)| (m, f))
                };
                blocks.push(quantize_block(&d, qn, factor));
            }
        }
        Comp {
            id,
            h: hv,
            v: hv,
            tq,
            bw,
            bh,
            nbw: p.w.div_ceil(8),
            nbh: p.h.div_ceil(8),
            blocks,
        }
    };
    let hv = if sub { 2 } else { 1 };
    let cscale = if sub { 2 } else { 1 };
    let comps = vec![
        make(&yplane, 1, hv, 0, &lum_nat, 1),
        make(&cplanes[0], 2, 1, 1, &chrom_nat, cscale),
        make(&cplanes[1], 3, 1, 1, &chrom_nat, cscale),
    ];

    let mut out = vec![0xFF, 0xD8];
    let dens = if params.dpi.is_finite() && params.dpi >= 1.0 {
        params.dpi.round().min(65535.0) as u16
    } else {
        72
    };
    let mut app0 = b"JFIF\0\x01\x01\x01".to_vec();
    app0.extend_from_slice(&dens.to_be_bytes());
    app0.extend_from_slice(&dens.to_be_bytes());
    app0.extend_from_slice(&[0, 0]);
    marker(&mut out, 0xE0, &app0);
    let mut dqt = vec![0u8];
    dqt.extend(qt.lum.iter().map(|&v| v as u8));
    dqt.push(1);
    dqt.extend(qt.chrom.iter().map(|&v| v as u8));
    marker(&mut out, 0xDB, &dqt);
    let mut sof = vec![8];
    sof.extend_from_slice(&(h as u16).to_be_bytes());
    sof.extend_from_slice(&(w as u16).to_be_bytes());
    sof.push(3);
    for c in &comps {
        sof.extend_from_slice(&[c.id, ((c.h as u8) << 4) | c.v as u8, c.tq as u8]);
    }
    marker(&mut out, if params.progressive { 0xC2 } else { 0xC0 }, &sof);

    if !params.progressive {
        let run = |sink: &mut dyn FnMut(&Comp, &[i32; 64], usize)| {
            for my in 0..mcuy {
                for mx in 0..mcux {
                    for (ci, c) in comps.iter().enumerate() {
                        for vy in 0..c.v {
                            for vx in 0..c.h {
                                sink(c, c.block(mx * c.h + vx, my * c.v + vy), ci);
                            }
                        }
                    }
                }
            }
        };
        let tables_v: Vec<Huff> = if params.optimize {
            let mut st = Stats([[0; 256]; 8]);
            let mut preds = [0i32; 3];
            run(&mut |_c, blk, ci| {
                let t = if ci == 0 { 0 } else { 1 };
                encode_block(&mut st, blk, &mut preds[ci], t, 2 + t);
            });
            vec![
                Huff::optimal(&st.0[0]),
                Huff::optimal(&st.0[1]),
                Huff::optimal(&st.0[2]),
                Huff::optimal(&st.0[3]),
            ]
        } else {
            vec![std_dc_lum(), std_dc_chrom(), std_ac_lum(), std_ac_chrom()]
        };
        dht(&mut out, 0x00, &tables_v[0]);
        dht(&mut out, 0x01, &tables_v[1]);
        dht(&mut out, 0x10, &tables_v[2]);
        dht(&mut out, 0x11, &tables_v[3]);
        marker(&mut out, 0xDA, &[3, 1, 0x00, 2, 0x11, 3, 0x11, 0, 63, 0]);
        let mut bw = BitWriter::new();
        {
            let mut em = Emit {
                bw: &mut bw,
                tables: &tables_v,
            };
            let mut preds = [0i32; 3];
            run(&mut |_c, blk, ci| {
                let t = if ci == 0 { 0 } else { 1 };
                encode_block(&mut em, blk, &mut preds[ci], t, 2 + t);
            });
        }
        bw.flush();
        out.extend_from_slice(&bw.out);
    } else {
        encode_progressive(&mut out, &comps, mcux, mcuy);
    }
    out.extend_from_slice(&[0xFF, 0xD9]);
    Ok(out)
}

/// Progressiv wie IJG `jpeg_simple_progression` (YCbCr, 10 Scans mit Spektralauswahl und
/// schrittweiser Annäherung): DC Al 1; Y AC 1–5 Al 2; Cr, Cb AC 1–63 Al 1; Y AC 6–63 Al 2;
/// Y AC 1–63 Ah 2 Al 1; DC Verfeinerung; Cr, Cb AC Verfeinerung; Y AC Verfeinerung. Huffman-Tabellen
/// je Scan optimiert (`optimize_coding`), Kodierung wie `jcphuff.c` (EOB-Läufe, Korrekturbits).
fn encode_progressive(out: &mut Vec<u8>, comps: &[Comp], mcux: usize, mcuy: usize) {
    // (Komponenten, Ss, Se, Ah, Al)
    let script: [(&[usize], usize, usize, u32, u32); 10] = [
        (&[0, 1, 2], 0, 0, 0, 1),
        (&[0], 1, 5, 0, 2),
        (&[2], 1, 63, 0, 1),
        (&[1], 1, 63, 0, 1),
        (&[0], 6, 63, 0, 2),
        (&[0], 1, 63, 2, 1),
        (&[0, 1, 2], 0, 0, 1, 0),
        (&[2], 1, 63, 1, 0),
        (&[1], 1, 63, 1, 0),
        (&[0], 1, 63, 1, 0),
    ];
    for (cs, ss, se, ah, al) in script {
        if ss == 0 {
            if ah == 0 {
                let walk = |s: &mut dyn Sink| {
                    let mut preds = [0i32; 3];
                    for my in 0..mcuy {
                        for mx in 0..mcux {
                            for &ci in cs {
                                let c = &comps[ci];
                                for vy in 0..c.v {
                                    for vx in 0..c.h {
                                        let v = c.block(mx * c.h + vx, my * c.v + vy)[0] >> al;
                                        let diff = v - preds[ci];
                                        preds[ci] = v;
                                        let (n, bits) = magnitude(diff);
                                        s.sym(if ci == 0 { 0 } else { 1 }, n as u8);
                                        s.raw(bits, n);
                                    }
                                }
                            }
                        }
                    }
                };
                let mut st = Stats([[0; 256]; 8]);
                walk(&mut st);
                let t = vec![Huff::optimal(&st.0[0]), Huff::optimal(&st.0[1])];
                dht(out, 0x00, &t[0]);
                dht(out, 0x01, &t[1]);
                marker(
                    out,
                    0xDA,
                    &[3, 1, 0x00, 2, 0x11, 3, 0x11, 0, 0, (ah << 4 | al) as u8],
                );
                let mut bw = BitWriter::new();
                walk(&mut Emit {
                    bw: &mut bw,
                    tables: &t,
                });
                bw.flush();
                out.extend_from_slice(&bw.out);
            } else {
                // DC-Verfeinerung: je Block ein Bit, ohne Huffman
                marker(
                    out,
                    0xDA,
                    &[3, 1, 0x00, 2, 0x11, 3, 0x11, 0, 0, (ah << 4 | al) as u8],
                );
                let mut bw = BitWriter::new();
                for my in 0..mcuy {
                    for mx in 0..mcux {
                        for &ci in cs {
                            let c = &comps[ci];
                            for vy in 0..c.v {
                                for vx in 0..c.h {
                                    bw.put(
                                        ((c.block(mx * c.h + vx, my * c.v + vy)[0] >> al) & 1)
                                            as u32,
                                        1,
                                    );
                                }
                            }
                        }
                    }
                }
                bw.flush();
                out.extend_from_slice(&bw.out);
            }
            continue;
        }
        let c = &comps[cs[0]];
        let walk = |s: &mut dyn Sink| {
            if ah == 0 {
                ac_first(s, c, ss, se, al)
            } else {
                ac_refine(s, c, ss, se, al)
            }
        };
        let mut st = Stats([[0; 256]; 8]);
        walk(&mut st);
        let t = vec![Huff::optimal(&st.0[0])];
        dht(out, 0x10, &t[0]);
        marker(
            out,
            0xDA,
            &[1, c.id, 0x00, ss as u8, se as u8, (ah << 4 | al) as u8],
        );
        let mut bw = BitWriter::new();
        walk(&mut Emit {
            bw: &mut bw,
            tables: &t,
        });
        bw.flush();
        out.extend_from_slice(&bw.out);
    }
}

/// EOB-Lauf ausgeben (`emit_eobrun`), danach die gepufferten Korrekturbits.
fn emit_eobrun(s: &mut dyn Sink, eobrun: &mut u32, be: &mut Vec<u8>) {
    if *eobrun > 0 {
        let n = 31 - eobrun.leading_zeros();
        s.sym(0, (n << 4) as u8);
        if n > 0 {
            s.raw(*eobrun & ((1 << n) - 1), n);
        }
        *eobrun = 0;
        for &b in be.iter() {
            s.raw(b as u32, 1);
        }
        be.clear();
    }
}

/// Erster AC-Scan einer Komponente (`encode_mcu_AC_first`).
fn ac_first(s: &mut dyn Sink, c: &Comp, ss: usize, se: usize, al: u32) {
    let mut eobrun = 0u32;
    let mut be = Vec::new();
    for by in 0..c.nbh {
        for bx in 0..c.nbw {
            let blk = c.block(bx, by);
            let mut r = 0u32;
            for &zi in &ZIGZAG[ss..=se] {
                let v = blk[zi];
                let (mag, bits) = if v < 0 {
                    let m = (-v) >> al;
                    (m, !m)
                } else {
                    let m = v >> al;
                    (m, m)
                };
                if mag == 0 {
                    r += 1;
                    continue;
                }
                emit_eobrun(s, &mut eobrun, &mut be);
                while r > 15 {
                    s.sym(0, 0xF0);
                    r -= 16;
                }
                let n = nbits(mag);
                s.sym(0, ((r << 4) | n) as u8);
                s.raw(bits as u32 & ((1u32 << n) - 1), n);
                r = 0;
            }
            if r > 0 {
                eobrun += 1;
                if eobrun == 0x7FFF {
                    emit_eobrun(s, &mut eobrun, &mut be);
                }
            }
        }
    }
    emit_eobrun(s, &mut eobrun, &mut be);
}

/// AC-Verfeinerungsscan einer Komponente (`encode_mcu_AC_refine`).
fn ac_refine(s: &mut dyn Sink, c: &Comp, ss: usize, se: usize, al: u32) {
    const MAX_CORR_BITS: usize = 1000;
    let mut eobrun = 0u32;
    let mut be: Vec<u8> = Vec::new();
    let mut abs = [0i32; 64];
    for by in 0..c.nbh {
        for bx in 0..c.nbw {
            let blk = c.block(bx, by);
            let mut eob = 0usize;
            for k in ss..=se {
                let t = blk[ZIGZAG[k]].abs() >> al;
                abs[k] = t;
                if t == 1 {
                    eob = k;
                }
            }
            let mut r = 0u32;
            let mut br: Vec<u8> = Vec::new();
            for k in ss..=se {
                let t = abs[k];
                if t == 0 {
                    r += 1;
                    continue;
                }
                while r > 15 && k <= eob {
                    emit_eobrun(s, &mut eobrun, &mut be);
                    s.sym(0, 0xF0);
                    r -= 16;
                    for &b in &br {
                        s.raw(b as u32, 1);
                    }
                    br.clear();
                }
                if t > 1 {
                    br.push((t & 1) as u8);
                    continue;
                }
                emit_eobrun(s, &mut eobrun, &mut be);
                s.sym(0, ((r << 4) + 1) as u8);
                s.raw(if blk[ZIGZAG[k]] < 0 { 0 } else { 1 }, 1);
                for &b in &br {
                    s.raw(b as u32, 1);
                }
                br.clear();
                r = 0;
            }
            if r > 0 || !br.is_empty() {
                eobrun += 1;
                be.extend_from_slice(&br);
                if eobrun == 0x7FFF || be.len() > MAX_CORR_BITS - 64 + 1 {
                    emit_eobrun(s, &mut eobrun, &mut be);
                }
            }
        }
    }
    emit_eobrun(s, &mut eobrun, &mut be);
}

/// Liest die DQT-Tabellen einer JPEG-Datei (Zickzack): `(Tabellen-ID, Werte)`.
pub fn read_dqt(data: &[u8]) -> Vec<(u8, Vec<u16>)> {
    let mut res = Vec::new();
    if data.get(..2) != Some(&[0xFF, 0xD8]) {
        return res;
    }
    let mut i = 2;
    while i + 4 <= data.len() {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }
        let m = data[i + 1];
        if m == 0xD8 || m == 0x01 || (0xD0..=0xD7).contains(&m) || m == 0xFF {
            i += if m == 0xFF { 1 } else { 2 };
            continue;
        }
        let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        let seg = data
            .get(i + 4..(i + 2 + len).min(data.len()))
            .unwrap_or(&[]);
        if m == 0xDB {
            let mut j = 0;
            while j < seg.len() {
                let (pq, tq) = (seg[j] >> 4, seg[j] & 15);
                if pq == 0 {
                    if let Some(v) = seg.get(j + 1..j + 65) {
                        res.push((tq, v.iter().map(|&x| x as u16).collect()));
                    }
                    j += 65;
                } else {
                    if let Some(v) = seg.get(j + 1..j + 129) {
                        res.push((
                            tq,
                            v.chunks(2)
                                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                                .collect(),
                        ));
                    }
                    j += 129;
                }
            }
        }
        if m == 0xDA {
            break;
        }
        i += 2 + len;
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fdct_dc_only() {
        let mut d = [10i32; 64];
        fdct_islow(&mut d);
        assert_eq!(d[0], 10 * 64);
        assert!(d[1..].iter().all(|&v| v == 0));
    }

    #[test]
    fn quantization_rule() {
        let mut coef = [0i32; 64];
        coef[0] = 8 * 16 * 3 + 30; // 3,23 Schritte → 3 (Viertel-Schritt-Rundung)
        coef[1] = -(8 * 10 + 25); // −1,31 → −1
        let mut q = [1u16; 64];
        q[0] = 16;
        q[1] = 10;
        let r = quantize_block(&coef, &q, None);
        assert_eq!(r[0], 3);
        assert_eq!(r[1], -1);
        // 0,8 Schritte: IJG rundet auf 1, das Original ebenfalls ((s/4 + 0,8 s) div s = 1)
        coef[2] = 64;
        q[2] = 10;
        assert_eq!(quantize_block(&coef, &q, None)[2], 1);
        coef[2] = 40; // 0,5 → 0 (Totzone)
        assert_eq!(quantize_block(&coef, &q, None)[2], 0);
    }

    #[test]
    fn ycc_conversion() {
        assert_eq!(rgb_to_ycc(255, 255, 255), (255, 128, 128));
        assert_eq!(rgb_to_ycc(0, 0, 0), (0, 128, 128));
        assert_eq!(rgb_to_ycc(255, 0, 0).0, 76);
    }

    #[test]
    fn encode_and_dqt() {
        let t = FwJpegTables::ijg_fallback();
        let px: Vec<u32> = (0..40 * 30)
            .map(|i| px::argb(255, (i % 40 * 6) as u32, (i / 40 * 8) as u32, 128))
            .collect();
        for (prog, sub) in [(false, 0), (false, 1), (true, 0), (true, 1)] {
            let p = JpegParams {
                quality: 60,
                subsampling: sub,
                progressive: prog,
                ..Default::default()
            };
            let j = encode_jpeg(&px, 40, 30, &p, &t).unwrap();
            assert_eq!(&j[..2], &[0xFF, 0xD8]);
            let dqt = read_dqt(&j);
            assert_eq!(dqt.len(), 2);
            assert_eq!(dqt[0].1, t.qtables(60).lum.to_vec());
        }
        assert!(encode_jpeg(&[], 0, 0, &JpegParams::default(), &t).is_err());
    }
}
