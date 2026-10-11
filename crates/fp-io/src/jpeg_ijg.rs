//! Eigener JPEG-Dekoder mit dem Rechenweg der IJG-Bibliothek (Fassung 6b), wie ihn das Original
//! beim Import nutzt: Huffman-Basis- und progressive Kodierung (SOF0/1/2), 8 Bit, 1/3/4
//! Komponenten, Neustartmarken; ganzzahlige inverse DCT `jpeg_idct_islow` mit Bereichstabelle
//! (`RANGE_MASK`), „fancy upsampling“ für h2v1/h2v2 (Dreiecksfilter, nur bei Breite > 2), sonst
//! Wiederholung; YCbCr → RGB mit den Festkomma-Tabellen von `jdcolor.c`.
//!
//! Arithmetische Kodierung, 12 Bit und verlustfreies JPEG liefern einen Fehler; der Aufrufer
//! weicht dann auf zune-jpeg aus. Keine Panik auf beschädigten Daten.

/// Farbraum der Komponenten.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JpegColor {
    Gray,
    YCbCr,
    Rgb,
    Cmyk,
    Ycck,
}

/// Dekodiertes Bild: volle Auflösung je Komponente (`width·height` Bytes), noch ohne Farbumrechnung.
#[derive(Clone, Debug)]
pub struct JpegPlanes {
    pub width: u32,
    pub height: u32,
    pub color: JpegColor,
    pub planes: Vec<Vec<u8>>,
    /// APP14-Transform-Segment vorhanden (erkannt am Aufbau: Marker `0xEE`, mindestens 12 Byte).
    pub app14: bool,
}

const ZIGZAG: [usize; 64 + 16] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20, 13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59, 52, 45, 38,
    31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63, // Schutz wie `jpeg_natural_order` (Überlauf)
    63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63,
];

type R<T> = std::result::Result<T, String>;

#[derive(Clone, Default)]
struct Huff {
    /// (Länge, Wert) je 9-Bit-Präfix; Länge 0 = langsamer Weg.
    look: Vec<(u8, u8)>,
    maxcode: [i32; 18],
    valoffset: [i32; 17],
    values: Vec<u8>,
}

impl Huff {
    fn new(counts: &[u8; 16], values: Vec<u8>) -> R<Huff> {
        let mut h = Huff { look: vec![(0, 0); 512], maxcode: [-1; 18], valoffset: [0; 17], values };
        let mut code = 0i32;
        let mut k = 0usize;
        for l in 1..=16usize {
            let n = counts[l - 1] as usize;
            if n > 0 {
                h.valoffset[l] = k as i32 - code;
                for _ in 0..n {
                    if l <= 9 {
                        let shift = 9 - l;
                        let base = (code << shift) as usize;
                        for j in 0..(1usize << shift) {
                            if let (Some(slot), Some(&v)) = (h.look.get_mut(base + j), h.values.get(k)) {
                                *slot = (l as u8, v);
                            }
                        }
                    }
                    code += 1;
                    k += 1;
                }
                h.maxcode[l] = code - 1;
            }
            code <<= 1;
            if code > (1 << 17) {
                return Err("Huffman-Tabelle ungültig".into());
            }
        }
        h.maxcode[17] = i32::MAX;
        Ok(h)
    }
}

struct Bits<'a> {
    d: &'a [u8],
    p: usize,
    acc: u64,
    n: u32,
    /// Gefundene Marke (Ende der Daten des Scans).
    marker: Option<u8>,
}

impl<'a> Bits<'a> {
    fn new(d: &'a [u8], p: usize) -> Self {
        Bits { d, p, acc: 0, n: 0, marker: None }
    }
    fn fill(&mut self) {
        while self.n <= 56 {
            let mut b = 0u8;
            if self.marker.is_none() && self.p < self.d.len() {
                b = self.d[self.p];
                if b == 0xFF {
                    let nx = self.d.get(self.p + 1).copied().unwrap_or(0xD9);
                    if nx == 0 {
                        self.p += 2;
                    } else {
                        self.marker = Some(nx);
                        b = 0;
                    }
                } else {
                    self.p += 1;
                }
            }
            self.acc |= (b as u64) << (56 - self.n);
            self.n += 8;
        }
    }
    fn peek(&mut self, k: u32) -> u32 {
        if self.n < k {
            self.fill();
        }
        (self.acc >> (64 - k)) as u32
    }
    fn skip(&mut self, k: u32) {
        self.acc <<= k;
        self.n -= k;
    }
    fn get(&mut self, k: u32) -> i32 {
        if k == 0 {
            return 0;
        }
        let v = self.peek(k);
        self.skip(k);
        v as i32
    }
    fn bit(&mut self) -> bool {
        self.get(1) != 0
    }
    fn decode(&mut self, h: &Huff) -> R<u8> {
        if self.n < 16 {
            self.fill();
        }
        let p9 = self.peek(9) as usize;
        if let Some(&(l, v)) = h.look.get(p9)
            && l > 0
        {
            self.skip(l as u32);
            return Ok(v);
        }
        let mut code = 0i32;
        for l in 1..=16usize {
            code = (code << 1) | self.get(1);
            if code <= h.maxcode[l] {
                let idx = (code + h.valoffset[l]) as usize;
                return h.values.get(idx).copied().ok_or_else(|| "Huffman-Wert fehlt".to_string());
            }
        }
        Err("ungültiger Huffman-Code".into())
    }
    fn receive_extend(&mut self, s: u32) -> i32 {
        if s == 0 {
            return 0;
        }
        let v = self.get(s);
        if v < (1 << (s - 1)) { v - (1 << s) + 1 } else { v }
    }
    /// Neustart: Bitpuffer leeren, Marke RSTn überspringen.
    fn restart(&mut self) {
        self.acc = 0;
        self.n = 0;
        if let Some(m) = self.marker
            && (0xD0..=0xD7).contains(&m)
        {
            self.p += 2;
            self.marker = None;
            return;
        }
        // Marke noch nicht gelesen: bis zur nächsten RST suchen
        while self.p + 1 < self.d.len() {
            if self.d[self.p] == 0xFF && (0xD0..=0xD7).contains(&self.d[self.p + 1]) {
                self.p += 2;
                break;
            }
            if self.d[self.p] == 0xFF && self.d[self.p + 1] != 0 && self.d[self.p + 1] != 0xFF {
                break;
            }
            self.p += 1;
        }
        self.marker = None;
    }
    /// Position hinter den Scan-Daten (Beginn der nächsten Marke).
    fn end(&self) -> usize {
        let mut p = self.p;
        while p + 1 < self.d.len() && !(self.d[p] == 0xFF && self.d[p + 1] != 0 && !(0xD0..=0xD7).contains(&self.d[p + 1])) {
            p += 1;
        }
        p
    }
}

#[derive(Clone, Default)]
struct Comp {
    id: u8,
    h: usize,
    v: usize,
    tq: usize,
    bw: usize,
    bh: usize,
    coefs: Vec<i32>,
    dc_pred: i32,
}

/// Dekodiert die Datei in Komponentenebenen voller Auflösung.
pub fn decode_planes(d: &[u8]) -> R<JpegPlanes> {
    if d.len() < 4 || d[0] != 0xFF || d[1] != 0xD8 {
        return Err("kein JPEG".into());
    }
    let mut qt = [[0i32; 64]; 4];
    let mut dc = vec![Huff::default(); 4];
    let mut ac = vec![Huff::default(); 4];
    let mut comps: Vec<Comp> = Vec::new();
    let (mut width, mut height) = (0usize, 0usize);
    let (mut hmax, mut vmax) = (1usize, 1usize);
    let mut progressive = false;
    let mut restart = 0usize;
    let mut app14: Option<u8> = None;
    let mut jfif = false;
    let mut p = 2usize;
    let mut frame = false;
    loop {
        // nächste Marke suchen
        while p < d.len() && d[p] != 0xFF {
            p += 1;
        }
        while p < d.len() && d[p] == 0xFF {
            p += 1;
        }
        let Some(&m) = d.get(p) else { break };
        p += 1;
        if m == 0xD9 {
            break;
        }
        if (0xD0..=0xD7).contains(&m) || m == 0x01 {
            continue;
        }
        let len = d.get(p..p + 2).map(|b| u16::from_be_bytes([b[0], b[1]]) as usize).ok_or("Segment abgeschnitten")?;
        let seg = d.get(p + 2..p + len).ok_or("Segment abgeschnitten")?;
        let next = p + len;
        match m {
            0xDB => {
                let mut i = 0;
                while i < seg.len() {
                    let pq = seg[i] >> 4;
                    let t = (seg[i] & 3) as usize;
                    i += 1;
                    for k in 0..64 {
                        let v = if pq == 0 {
                            *seg.get(i + k).ok_or("DQT")? as i32
                        } else {
                            u16::from_be_bytes([*seg.get(i + 2 * k).ok_or("DQT")?, *seg.get(i + 2 * k + 1).ok_or("DQT")?]) as i32
                        };
                        qt[t][ZIGZAG[k]] = v;
                    }
                    i += if pq == 0 { 64 } else { 128 };
                }
            }
            0xC4 => {
                let mut i = 0;
                while i + 17 <= seg.len() {
                    let tc = seg[i] >> 4;
                    let th = (seg[i] & 3) as usize;
                    let mut counts = [0u8; 16];
                    counts.copy_from_slice(&seg[i + 1..i + 17]);
                    let n: usize = counts.iter().map(|&c| c as usize).sum();
                    let vals = seg.get(i + 17..i + 17 + n).ok_or("DHT")?.to_vec();
                    let h = Huff::new(&counts, vals)?;
                    if tc == 0 { dc[th] = h } else { ac[th] = h }
                    i += 17 + n;
                }
            }
            0xC0..=0xC2 => {
                if seg.len() < 6 || seg[0] != 8 {
                    return Err("nur 8-Bit-JPEG".into());
                }
                progressive = m == 0xC2;
                height = u16::from_be_bytes([seg[1], seg[2]]) as usize;
                width = u16::from_be_bytes([seg[3], seg[4]]) as usize;
                let n = seg[5] as usize;
                if width == 0 || height == 0 || !(n == 1 || n == 3 || n == 4) {
                    return Err("JPEG-Kopf ungültig".into());
                }
                if (width as u64) * (height as u64) > crate::MAX_PIXELS {
                    return Err("zu groß".into());
                }
                for k in 0..n {
                    let c = seg.get(6 + 3 * k..9 + 3 * k).ok_or("SOF")?;
                    let (h, v) = ((c[1] >> 4) as usize, (c[1] & 15) as usize);
                    if !(1..=4).contains(&h) || !(1..=4).contains(&v) {
                        return Err("Abtastfaktor".into());
                    }
                    comps.push(Comp { id: c[0], h, v, tq: (c[2] & 3) as usize, ..Comp::default() });
                }
                hmax = comps.iter().map(|c| c.h).max().unwrap_or(1);
                vmax = comps.iter().map(|c| c.v).max().unwrap_or(1);
                let mcux = width.div_ceil(8 * hmax);
                let mcuy = height.div_ceil(8 * vmax);
                for c in &mut comps {
                    c.bw = mcux * c.h;
                    c.bh = mcuy * c.v;
                    c.coefs = vec![0; c.bw * c.bh * 64];
                }
                frame = true;
            }
            0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => return Err("JPEG-Variante nicht unterstützt (arithmetisch/verlustfrei)".into()),
            0xDD => restart = seg.get(..2).map(|b| u16::from_be_bytes([b[0], b[1]]) as usize).unwrap_or(0),
            0xE0 => jfif |= seg.starts_with(b"JFIF\0"),
            0xEE => {
                if seg.len() >= 12 {
                    app14 = Some(seg[11]);
                }
            }
            0xDA => {
                if !frame {
                    return Err("Scan vor dem Bildkopf".into());
                }
                let ns = *seg.first().ok_or("SOS")? as usize;
                let mut sel: Vec<(usize, usize, usize)> = Vec::new();
                for k in 0..ns {
                    let cs = seg.get(1 + 2 * k..3 + 2 * k).ok_or("SOS")?;
                    let ci = comps.iter().position(|c| c.id == cs[0]).ok_or("SOS-Komponente")?;
                    sel.push((ci, (cs[1] >> 4) as usize & 3, (cs[1] & 3) as usize));
                }
                let tail = seg.get(1 + 2 * ns..4 + 2 * ns).ok_or("SOS")?;
                let (ss, se, ah, al) = (tail[0] as usize, tail[1] as usize, (tail[2] >> 4) as u32, (tail[2] & 15) as u32);
                if se > 63 || ss > se {
                    return Err("Spektralauswahl".into());
                }
                let mut bits = Bits::new(d, next);
                for c in &mut comps {
                    c.dc_pred = 0;
                }
                let mut eobrun = 0u32;
                // Blockfolge des Scans
                let single = sel.len() == 1;
                let mcux = width.div_ceil(8 * hmax);
                let mcuy = height.div_ceil(8 * vmax);
                let units: usize = if single {
                    let c = &comps[sel[0].0];
                    let cw = (width * c.h).div_ceil(hmax);
                    let ch = (height * c.v).div_ceil(vmax);
                    cw.div_ceil(8) * ch.div_ceil(8)
                } else {
                    mcux * mcuy
                };
                let (sw, sh_) = if single {
                    let c = &comps[sel[0].0];
                    ((width * c.h).div_ceil(hmax).div_ceil(8), (height * c.v).div_ceil(vmax).div_ceil(8))
                } else {
                    (mcux, mcuy)
                };
                let mut todo = restart;
                for u in 0..units {
                    if restart > 0 {
                        if todo == 0 {
                            bits.restart();
                            for c in &mut comps {
                                c.dc_pred = 0;
                            }
                            eobrun = 0;
                            todo = restart;
                        }
                        todo -= 1;
                    }
                    let (ux, uy) = (u % sw, u / sw);
                    let _ = sh_;
                    for &(ci, td, ta) in &sel {
                        let (bh_, bv_) = if single { (1, 1) } else { (comps[ci].h, comps[ci].v) };
                        for by in 0..bv_ {
                            for bx in 0..bh_ {
                                let (x, y) = if single { (ux, uy) } else { (ux * comps[ci].h + bx, uy * comps[ci].v + by) };
                                let c = &mut comps[ci];
                                if x >= c.bw || y >= c.bh {
                                    continue;
                                }
                                let off = (y * c.bw + x) * 64;
                                let blk = &mut c.coefs[off..off + 64];
                                if !progressive {
                                    let s = bits.decode(&dc[td])? as u32;
                                    c.dc_pred += bits.receive_extend(s);
                                    blk[0] = c.dc_pred;
                                    let mut k = 1;
                                    while k < 64 {
                                        let rs = bits.decode(&ac[ta])?;
                                        let (r, s) = ((rs >> 4) as usize, (rs & 15) as u32);
                                        if s != 0 {
                                            k += r;
                                            if k > 63 {
                                                break;
                                            }
                                            blk[ZIGZAG[k]] = bits.receive_extend(s);
                                            k += 1;
                                        } else if r == 15 {
                                            k += 16;
                                        } else {
                                            break;
                                        }
                                    }
                                } else if ss == 0 {
                                    if ah == 0 {
                                        let s = bits.decode(&dc[td])? as u32;
                                        c.dc_pred += bits.receive_extend(s);
                                        blk[0] = c.dc_pred << al;
                                    } else if bits.bit() {
                                        blk[0] |= 1 << al;
                                    }
                                } else if ah == 0 {
                                    if eobrun > 0 {
                                        eobrun -= 1;
                                        continue;
                                    }
                                    let mut k = ss;
                                    while k <= se {
                                        let rs = bits.decode(&ac[ta])?;
                                        let (r, s) = ((rs >> 4) as usize, (rs & 15) as u32);
                                        if s != 0 {
                                            k += r;
                                            if k > 63 {
                                                break;
                                            }
                                            blk[ZIGZAG[k]] = bits.receive_extend(s) * (1 << al);
                                        } else if r < 15 {
                                            eobrun = (1u32 << r) - 1;
                                            if r > 0 {
                                                eobrun += bits.get(r as u32) as u32;
                                            }
                                            break;
                                        } else {
                                            k += 15;
                                        }
                                        k += 1;
                                    }
                                } else {
                                    refine_ac(&mut bits, &ac[ta], blk, ss, se, al, &mut eobrun)?;
                                }
                            }
                        }
                    }
                }
                p = bits.end();
                continue;
            }
            _ => {}
        }
        p = next;
    }
    if !frame {
        return Err("JPEG ohne Bild".into());
    }
    // inverse DCT und Hochtasten
    let n = comps.len();
    let mut planes = Vec::with_capacity(n);
    for c in &comps {
        let q = &qt[c.tq];
        let pw = c.bw * 8;
        let mut plane = vec![0u8; pw * c.bh * 8];
        for by in 0..c.bh {
            for bx in 0..c.bw {
                let off = (by * c.bw + bx) * 64;
                let mut out = [0u8; 64];
                idct_islow(&c.coefs[off..off + 64], q, &mut out);
                for r in 0..8 {
                    let o = (by * 8 + r) * pw + bx * 8;
                    plane[o..o + 8].copy_from_slice(&out[r * 8..r * 8 + 8]);
                }
            }
        }
        let dw = (width * c.h).div_ceil(hmax);
        let dh = (height * c.v).div_ceil(vmax);
        planes.push(upsample(&plane, pw, dw, dh, hmax / c.h, vmax / c.v, hmax % c.h == 0 && vmax % c.v == 0, width, height)?);
    }
    let color = match n {
        1 => JpegColor::Gray,
        3 => {
            if let Some(t) = app14 {
                if t == 0 { JpegColor::Rgb } else { JpegColor::YCbCr }
            } else if !jfif && comps.iter().map(|c| c.id).eq(*b"RGB") {
                JpegColor::Rgb
            } else {
                JpegColor::YCbCr
            }
        }
        _ => {
            if app14 == Some(2) { JpegColor::Ycck } else { JpegColor::Cmyk }
        }
    };
    Ok(JpegPlanes { width: width as u32, height: height as u32, color, planes, app14: app14.is_some() })
}

/// Verfeinerungsscan der AC-Koeffizienten (`decode_mcu_AC_refine`, jdphuff.c).
fn refine_ac(bits: &mut Bits<'_>, h: &Huff, blk: &mut [i32], ss: usize, se: usize, al: u32, eobrun: &mut u32) -> R<()> {
    let p1 = 1i32 << al;
    let m1 = -1i32 << al;
    let mut k = ss;
    let corr = |bits: &mut Bits<'_>, v: &mut i32| {
        if bits.bit() && (*v & p1) == 0 {
            if *v >= 0 {
                *v += p1;
            } else {
                *v += m1;
            }
        }
    };
    if *eobrun == 0 {
        while k <= se {
            let rs = bits.decode(h)?;
            let mut r = (rs >> 4) as i32;
            let mut s = (rs & 15) as i32;
            if s != 0 {
                s = if bits.bit() { p1 } else { m1 };
            } else if r != 15 {
                *eobrun = 1u32 << r;
                if r > 0 {
                    *eobrun += bits.get(r as u32) as u32;
                }
                break;
            }
            loop {
                let z = ZIGZAG[k.min(63)];
                if blk[z] != 0 {
                    corr(bits, &mut blk[z]);
                } else {
                    r -= 1;
                    if r < 0 {
                        break;
                    }
                }
                k += 1;
                if k > se {
                    break;
                }
            }
            if s != 0 && k <= 63 {
                blk[ZIGZAG[k]] = s;
            }
            k += 1;
        }
    }
    if *eobrun > 0 {
        while k <= se {
            let z = ZIGZAG[k];
            if blk[z] != 0 {
                corr(bits, &mut blk[z]);
            }
            k += 1;
        }
        *eobrun -= 1;
    }
    Ok(())
}

// ---------------------------------------------------------------- inverse DCT (jidctint.c, islow)
const CONST_BITS: i32 = 13;
const PASS1_BITS: i32 = 2;
const F0_298631336: i64 = 2446;
const F0_390180644: i64 = 3196;
const F0_541196100: i64 = 4433;
const F0_765366865: i64 = 6270;
const F0_899976223: i64 = 7373;
const F1_175875602: i64 = 9633;
const F1_501321110: i64 = 12299;
const F1_847759065: i64 = 15137;
const F1_961570560: i64 = 16069;
const F2_053119869: i64 = 16819;
const F2_562915447: i64 = 20995;
const F3_072711026: i64 = 25172;

#[inline]
fn descale(x: i64, n: i32) -> i64 {
    (x + (1i64 << (n - 1))) >> n
}

/// Bereichsbegrenzung nach der IDCT wie `idct_range_limit` (Index `& 1023`, Mitte 128).
#[inline]
fn range_limit(v: i64) -> u8 {
    let i = (v & 1023) as i32;
    match i {
        0..=127 => (i + 128) as u8,
        128..=511 => 255,
        512..=895 => 0,
        _ => (i - 896) as u8,
    }
}

fn idct_islow(coef: &[i32], q: &[i32; 64], out: &mut [u8; 64]) {
    let mut ws = [0i64; 64];
    for col in 0..8 {
        let deq = |r: usize| coef[r * 8 + col] as i64 * q[r * 8 + col] as i64;
        if (1..8).all(|r| coef[r * 8 + col] == 0) {
            let dcval = deq(0) << PASS1_BITS;
            for r in 0..8 {
                ws[r * 8 + col] = dcval;
            }
            continue;
        }
        let (z2, z3) = (deq(2), deq(6));
        let z1 = (z2 + z3) * F0_541196100;
        let tmp2 = z1 + z3 * -F1_847759065;
        let tmp3 = z1 + z2 * F0_765366865;
        let (z2, z3) = (deq(0), deq(4));
        let tmp0 = (z2 + z3) << CONST_BITS;
        let tmp1 = (z2 - z3) << CONST_BITS;
        let (tmp10, tmp13, tmp11, tmp12) = (tmp0 + tmp3, tmp0 - tmp3, tmp1 + tmp2, tmp1 - tmp2);
        let (mut t0, mut t1, mut t2, mut t3) = (deq(7), deq(5), deq(3), deq(1));
        let (z1, z2, z3, z4) = (t0 + t3, t1 + t2, t0 + t2, t1 + t3);
        let z5 = (z3 + z4) * F1_175875602;
        t0 *= F0_298631336;
        t1 *= F2_053119869;
        t2 *= F3_072711026;
        t3 *= F1_501321110;
        let z1 = z1 * -F0_899976223;
        let z2 = z2 * -F2_562915447;
        let z3 = z3 * -F1_961570560 + z5;
        let z4 = z4 * -F0_390180644 + z5;
        t0 += z1 + z3;
        t1 += z2 + z4;
        t2 += z2 + z3;
        t3 += z1 + z4;
        let n = CONST_BITS - PASS1_BITS;
        ws[col] = descale(tmp10 + t3, n);
        ws[56 + col] = descale(tmp10 - t3, n);
        ws[8 + col] = descale(tmp11 + t2, n);
        ws[48 + col] = descale(tmp11 - t2, n);
        ws[16 + col] = descale(tmp12 + t1, n);
        ws[40 + col] = descale(tmp12 - t1, n);
        ws[24 + col] = descale(tmp13 + t0, n);
        ws[32 + col] = descale(tmp13 - t0, n);
    }
    let n = CONST_BITS + PASS1_BITS + 3;
    for row in 0..8 {
        let w = &ws[row * 8..row * 8 + 8];
        let o = &mut out[row * 8..row * 8 + 8];
        if w[1..].iter().all(|&v| v == 0) {
            let v = range_limit(descale(w[0], PASS1_BITS + 3));
            o.fill(v);
            continue;
        }
        let (z2, z3) = (w[2], w[6]);
        let z1 = (z2 + z3) * F0_541196100;
        let tmp2 = z1 + z3 * -F1_847759065;
        let tmp3 = z1 + z2 * F0_765366865;
        let tmp0 = (w[0] + w[4]) << CONST_BITS;
        let tmp1 = (w[0] - w[4]) << CONST_BITS;
        let (tmp10, tmp13, tmp11, tmp12) = (tmp0 + tmp3, tmp0 - tmp3, tmp1 + tmp2, tmp1 - tmp2);
        let (mut t0, mut t1, mut t2, mut t3) = (w[7], w[5], w[3], w[1]);
        let (z1, z2, z3, z4) = (t0 + t3, t1 + t2, t0 + t2, t1 + t3);
        let z5 = (z3 + z4) * F1_175875602;
        t0 *= F0_298631336;
        t1 *= F2_053119869;
        t2 *= F3_072711026;
        t3 *= F1_501321110;
        let z1 = z1 * -F0_899976223;
        let z2 = z2 * -F2_562915447;
        let z3 = z3 * -F1_961570560 + z5;
        let z4 = z4 * -F0_390180644 + z5;
        t0 += z1 + z3;
        t1 += z2 + z4;
        t2 += z2 + z3;
        t3 += z1 + z4;
        o[0] = range_limit(descale(tmp10 + t3, n));
        o[7] = range_limit(descale(tmp10 - t3, n));
        o[1] = range_limit(descale(tmp11 + t2, n));
        o[6] = range_limit(descale(tmp11 - t2, n));
        o[2] = range_limit(descale(tmp12 + t1, n));
        o[5] = range_limit(descale(tmp12 - t1, n));
        o[3] = range_limit(descale(tmp13 + t0, n));
        o[4] = range_limit(descale(tmp13 - t0, n));
    }
}

// ---------------------------------------------------------------- Hochtasten (jdsample.c)
#[allow(clippy::too_many_arguments)]
fn upsample(src: &[u8], stride: usize, dw: usize, dh: usize, hx: usize, vx: usize, integral: bool, w: usize, h: usize) -> R<Vec<u8>> {
    if !integral || hx == 0 || vx == 0 {
        return Err("nicht ganzzahliges Abtastverhältnis".into());
    }
    let row = |y: usize| -> &[u8] {
        let y = y.min(dh.saturating_sub(1));
        &src[y * stride..y * stride + stride]
    };
    let mut out = vec![0u8; w * h];
    let fancy = dw > 2;
    match (hx, vx) {
        (1, 1) => {
            for y in 0..h {
                out[y * w..y * w + w].copy_from_slice(&row(y)[..w]);
            }
        }
        (2, 1) if fancy => {
            let mut line = vec![0u8; dw * 2];
            for y in 0..h {
                let r = row(y);
                h2v1_fancy(&r[..dw], &mut line);
                out[y * w..y * w + w].copy_from_slice(&line[..w]);
            }
        }
        (2, 2) if fancy => {
            let mut line = vec![0u8; dw * 2];
            for y in 0..h {
                let iy = y / 2;
                let near = row(iy);
                let far = if y % 2 == 0 { row(iy.saturating_sub(if iy == 0 { 0 } else { 1 })) } else { row(iy + 1) };
                h2v2_fancy(&near[..dw], &far[..dw], &mut line);
                out[y * w..y * w + w].copy_from_slice(&line[..w]);
            }
        }
        _ => {
            for y in 0..h {
                let r = row(y / vx);
                for x in 0..w {
                    out[y * w + x] = r[(x / hx).min(stride - 1)];
                }
            }
        }
    }
    Ok(out)
}

fn h2v1_fancy(inp: &[u8], out: &mut [u8]) {
    let n = inp.len();
    let g = |i: usize| inp[i] as i32;
    out[0] = inp[0];
    out[1] = ((g(0) * 3 + g(1) + 2) >> 2) as u8;
    for i in 1..n - 1 {
        let v = g(i) * 3;
        out[2 * i] = ((v + g(i - 1) + 1) >> 2) as u8;
        out[2 * i + 1] = ((v + g(i + 1) + 2) >> 2) as u8;
    }
    out[2 * n - 2] = ((g(n - 1) * 3 + g(n - 2) + 1) >> 2) as u8;
    out[2 * n - 1] = inp[n - 1];
}

fn h2v2_fancy(near: &[u8], far: &[u8], out: &mut [u8]) {
    let n = near.len();
    let col = |i: usize| near[i] as i32 * 3 + far[i] as i32;
    let mut this = col(0);
    let mut next = col(1);
    out[0] = ((this * 4 + 8) >> 4) as u8;
    out[1] = ((this * 3 + next + 7) >> 4) as u8;
    let mut last = this;
    this = next;
    for i in 1..n - 1 {
        next = col(i + 1);
        out[2 * i] = ((this * 3 + last + 8) >> 4) as u8;
        out[2 * i + 1] = ((this * 3 + next + 7) >> 4) as u8;
        last = this;
        this = next;
    }
    out[2 * n - 2] = ((this * 3 + last + 8) >> 4) as u8;
    out[2 * n - 1] = ((this * 4 + 7) >> 4) as u8;
}

// ---------------------------------------------------------------- Farbe (jdcolor.c)
/// Festkomma-Tabellen von `build_ycc_rgb_table`.
pub struct YccTables {
    cr_r: [i32; 256],
    cb_b: [i32; 256],
    cr_g: [i64; 256],
    cb_g: [i64; 256],
}

impl Default for YccTables {
    fn default() -> Self {
        const SCALEBITS: i32 = 16;
        let one_half = 1i64 << (SCALEBITS - 1);
        let fix = |x: f64| (x * (1i64 << SCALEBITS) as f64 + 0.5) as i64;
        let mut t = YccTables { cr_r: [0; 256], cb_b: [0; 256], cr_g: [0; 256], cb_g: [0; 256] };
        for i in 0..256 {
            let x = i as i64 - 128;
            t.cr_r[i] = ((fix(1.40200) * x + one_half) >> SCALEBITS) as i32;
            t.cb_b[i] = ((fix(1.77200) * x + one_half) >> SCALEBITS) as i32;
            t.cr_g[i] = -fix(0.71414) * x;
            t.cb_g[i] = -fix(0.34414) * x + one_half;
        }
        t
    }
}

impl YccTables {
    /// YCbCr → RGB wie `ycc_rgb_convert`.
    #[inline]
    pub fn rgb(&self, y: u8, cb: u8, cr: u8) -> [u8; 3] {
        let y = y as i32;
        let c = |v: i32| v.clamp(0, 255) as u8;
        let g = ((self.cb_g[cb as usize] + self.cr_g[cr as usize]) >> 16) as i32;
        [c(y + self.cr_r[cr as usize]), c(y + g), c(y + self.cb_b[cb as usize])]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idct_dc_only_and_range() {
        let mut c = [0i32; 64];
        c[0] = 10;
        let q = [8i32; 64];
        let mut out = [0u8; 64];
        idct_islow(&c, &q, &mut out);
        // DC 80 ⇒ 80/8 = 10 über der Mitte
        assert!(out.iter().all(|&v| v == 138), "{out:?}");
        assert_eq!(range_limit(-128), 0);
        assert_eq!(range_limit(200), 255);
        assert_eq!(range_limit(-300), 0);
    }

    #[test]
    fn ycc_tables() {
        let t = YccTables::default();
        assert_eq!(t.rgb(128, 128, 128), [128, 128, 128]);
        assert_eq!(t.rgb(76, 85, 255), [254, 0, 0]);
        assert!(decode_planes(b"\xff\xd8\xff\xd9").is_err());
    }

    #[test]
    fn fancy_edges() {
        let mut o = [0u8; 6];
        h2v1_fancy(&[0, 100, 200], &mut o);
        assert_eq!(o, [0, 25, 75, 125, 175, 200]);
    }
}
