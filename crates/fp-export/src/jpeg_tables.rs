//! JPEG-Qualitätstabellen wie im Original (08 §4.2): zwölf Ankertabellen (Luminanz/Chrominanz für die
//! Stützstellen q = 1, 40, 60, 80, 90, 100) und lineare Interpolation mit Rundung dazwischen.
//!
//! Die Ankertabellen sind Daten der EXE des Originals (`.rdata 0x017cbe08…0x017cc908`) und werden **nicht**
//! eingebettet (wie die CMYK-Tabelle in `fp-color`): [`FwJpegTables::load_default`] liest sie zur
//! Laufzeit aus einer vorhandenen Installation (`FP_JPEG_TABLES` = Pfad zur Haupt-EXE oder zu
//! einer JSON-Datei `{"017cbe08": [64 Werte], …}`, sonst der Ressourcenordner des Originals aus
//! [`fp_color::install`]). Ohne
//! Installation werden die Anker aus den IJG-Standardtabellen (Anhang K der JPEG-Norm) mit der
//! IJG-Qualitätsskalierung berechnet ([`FwJpegTables::ijg_fallback`]); die Interpolation bleibt gleich.

/// Stützstellen: (Qualität, Adresse Luminanz, Adresse Chrominanz).
pub const ANCHORS: [(i32, u32, u32); 6] = [
    (1, 0x017c_c908, 0x017c_c808),
    (40, 0x017c_c708, 0x017c_c608),
    (60, 0x017c_c508, 0x017c_c408),
    (80, 0x017c_c308, 0x017c_c208),
    (90, 0x017c_c108, 0x017c_c008),
    (100, 0x017c_bf08, 0x017c_be08),
];

/// Zickzack-Reihenfolge: `ZIGZAG[k]` = natürlicher Index des k-ten Zickzack-Elements.
pub const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

/// IJG-Standardtabelle Luminanz (natürliche Reihenfolge, Anhang K.1).
const STD_LUM: [u32; 64] = [
    16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69, 56,
    14, 17, 22, 29, 51, 87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81, 104, 113,
    92, 49, 64, 78, 87, 103, 121, 120, 101, 72, 92, 95, 98, 112, 100, 103, 99,
];
/// IJG-Standardtabelle Chrominanz (natürliche Reihenfolge, Anhang K.2).
const STD_CHROM: [u32; 64] = [
    17, 18, 24, 47, 99, 99, 99, 99, 18, 21, 26, 66, 99, 99, 99, 99, 24, 26, 56, 99, 99, 99, 99, 99,
    47, 66, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
];

/// Herkunft der geladenen Ankertabellen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TableSource {
    /// Aus dem Ressourcenordner des Originals (bzw. einer Datei daraus).
    Original(String),
    /// IJG-Ersatz (kein Ressourcenordner des Originals gefunden).
    IjgFallback,
}

/// Die zwölf Ankertabellen (Zickzack-Reihenfolge wie im DQT-Segment).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FwJpegTables {
    /// `lum[i]`, `chrom[i]` für Stützstelle `ANCHORS[i]`.
    pub lum: [[u32; 64]; 6],
    pub chrom: [[u32; 64]; 6],
    pub source: TableSource,
}

/// Interpolierte Tabellen einer Qualitätsstufe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualityTables {
    /// Luminanz, Zickzack, auf 255 begrenzt.
    pub lum: [u16; 64],
    /// Chrominanz, Zickzack, auf 255 begrenzt.
    pub chrom: [u16; 64],
    /// Summe der ungekappten Luminanzwerte (Rückgabewert von `@00a5e940`, für selektives JPEG).
    pub lum_sum: i64,
}

fn ijg_scale(q: i32, base: &[u32; 64]) -> [u32; 64] {
    let q = q.clamp(1, 100);
    let s = if q < 50 { 5000 / q } else { 200 - q * 2 };
    let mut zz = [0u32; 64];
    for k in 0..64 {
        let v = ((base[ZIGZAG[k]] as i64 * s as i64 + 50) / 100).clamp(1, 255) as u32;
        zz[k] = if q == 100 { 1 } else { v };
    }
    zz
}

impl FwJpegTables {
    /// Ersatz aus den IJG-Standardtabellen (q = 100 ⇒ alle Werte 1 wie im Original).
    pub fn ijg_fallback() -> FwJpegTables {
        let mut lum = [[0u32; 64]; 6];
        let mut chrom = [[0u32; 64]; 6];
        for (i, &(q, _, _)) in ANCHORS.iter().enumerate() {
            lum[i] = ijg_scale(q, &STD_LUM);
            chrom[i] = ijg_scale(q, &STD_CHROM);
        }
        FwJpegTables {
            lum,
            chrom,
            source: TableSource::IjgFallback,
        }
    }

    /// Liest die Anker aus der Haupt-EXE des Originals (PE32, `.rdata`, je Tabelle 64 × `u32`).
    pub fn from_original_exe(exe: &[u8]) -> Option<FwJpegTables> {
        let pe = Pe::parse(exe)?;
        let read = |va: u32| -> Option<[u32; 64]> {
            let off = pe.file_offset(va)?;
            let bytes = exe.get(off..off + 256)?;
            let mut t = [0u32; 64];
            for (k, c) in bytes.as_chunks::<4>().0.iter().enumerate() {
                t[k] = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
            }
            t.iter().all(|&v| (1..=0xFFFF).contains(&v)).then_some(t)
        };
        let mut lum = [[0u32; 64]; 6];
        let mut chrom = [[0u32; 64]; 6];
        for (i, &(_, l, c)) in ANCHORS.iter().enumerate() {
            lum[i] = read(l)?;
            chrom[i] = read(c)?;
        }
        Some(FwJpegTables {
            lum,
            chrom,
            source: TableSource::Original("EXE".into()),
        })
    }

    /// Liest die Anker aus JSON `{"017cbe08": [64 Werte], …}` (Format von `re\export\_scratch`).
    pub fn from_json(text: &str) -> Option<FwJpegTables> {
        let find = |va: u32| -> Option<[u32; 64]> {
            let key = format!("\"{va:08x}\"");
            let start = text.find(&key)? + key.len();
            let rest = &text[start..];
            let open = rest.find('[')?;
            let close = rest.find(']')?;
            let vals: Vec<u32> = rest[open + 1..close]
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            vals.try_into().ok()
        };
        let mut lum = [[0u32; 64]; 6];
        let mut chrom = [[0u32; 64]; 6];
        for (i, &(_, l, c)) in ANCHORS.iter().enumerate() {
            lum[i] = find(l)?;
            chrom[i] = find(c)?;
        }
        Some(FwJpegTables {
            lum,
            chrom,
            source: TableSource::Original("JSON".into()),
        })
    }

    /// Lädt die Tabellen des Originals (Umgebungsvariable `FP_JPEG_TABLES`, dann Ressourcenordner),
    /// sonst [`FwJpegTables::ijg_fallback`].
    pub fn load_default() -> FwJpegTables {
        let mut paths: Vec<std::path::PathBuf> = Vec::new();
        if let Ok(p) = std::env::var("FP_JPEG_TABLES") {
            paths.push(p.into());
        }
        paths.extend(fp_color::install::original_exe());
        for p in paths {
            if let Some(t) = Self::from_path(&p) {
                return t;
            }
        }
        Self::ijg_fallback()
    }

    /// Lädt aus einer Datei (EXE oder JSON).
    pub fn from_path(p: &std::path::Path) -> Option<FwJpegTables> {
        let data = std::fs::read(p).ok()?;
        let mut t = if data.starts_with(b"MZ") {
            Self::from_original_exe(&data)?
        } else {
            Self::from_json(std::str::from_utf8(&data).ok()?)?
        };
        t.source = TableSource::Original(p.display().to_string());
        Some(t)
    }

    /// Tabellen einer Qualitätsstufe (`@00a5e940`): Abschnitte `[1,40) [40,60) [60,80) [80,90) [90,100]`,
    /// `T(q) = (T0·(q1−q) + T1·(q−q0) + (q1−q0) div 2) div (q1−q0)`, auf 255 begrenzt.
    pub fn qtables(&self, quality: i32) -> QualityTables {
        let q = quality.clamp(1, 100);
        let mut i = 0;
        while i < ANCHORS.len() - 2 && q >= ANCHORS[i + 1].0 {
            i += 1;
        }
        let (q0, q1) = (ANCHORS[i].0 as i64, ANCHORS[i + 1].0 as i64);
        let span = q1 - q0;
        let (w0, w1) = (q1 - q as i64, q as i64 - q0);
        let mut lum = [0u16; 64];
        let mut chrom = [0u16; 64];
        let mut sum = 0i64;
        for k in 0..64 {
            let a = (self.lum[i][k] as i64 * w0 + self.lum[i + 1][k] as i64 * w1 + span / 2) / span;
            let b =
                (self.chrom[i][k] as i64 * w0 + self.chrom[i + 1][k] as i64 * w1 + span / 2) / span;
            sum += a;
            lum[k] = a.clamp(1, 255) as u16;
            chrom[k] = b.clamp(1, 255) as u16;
        }
        QualityTables {
            lum,
            chrom,
            lum_sum: sum,
        }
    }

    /// Parameter für selektives JPEG (08 §4.5): `(q_tabellen, m, f)` oder `None` bei gleicher Qualität.
    pub fn selective_params(&self, q_main: i32, q_sel: i32) -> Option<(i32, i64, i64)> {
        let (qa, qb) = (q_main.clamp(1, 100), q_sel.clamp(1, 100));
        if qa == qb {
            return None;
        }
        let (hi, lo) = (qa.max(qb), qa.min(qb));
        let s_hi = self.qtables(hi).lum_sum.max(1);
        let s_lo = self.qtables(lo).lum_sum;
        let r = 2 * s_lo - s_hi;
        let m = (r.div_euclid(s_hi)).min(128);
        let f = ((r.rem_euclid(s_hi) << 6) / s_hi).clamp(1, 64);
        Some((hi, m, f))
    }
}

/// Minimaler PE-Leser: Abbildung virtueller Adressen auf Dateioffsets.
pub(crate) struct Pe {
    image_base: u64,
    sections: Vec<(u32, u32, u32, u32)>, // (va, vsize, raw_off, raw_size)
}

impl Pe {
    pub(crate) fn parse(d: &[u8]) -> Option<Pe> {
        let u16at = |o: usize| d.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
        let u32at = |o: usize| {
            d.get(o..o + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        if d.get(0..2)? != b"MZ" {
            return None;
        }
        let pe = u32at(0x3C)? as usize;
        if d.get(pe..pe + 4)? != b"PE\0\0" {
            return None;
        }
        let nsec = u16at(pe + 6)? as usize;
        let opt_size = u16at(pe + 20)? as usize;
        let opt = pe + 24;
        let magic = u16at(opt)?;
        let image_base = if magic == 0x20B {
            d.get(opt + 24..opt + 32)
                .map(|b| u64::from_le_bytes(b.try_into().unwrap_or([0; 8])))?
        } else {
            u32at(opt + 28)? as u64
        };
        let mut sections = Vec::new();
        let sh = opt + opt_size;
        for i in 0..nsec.min(96) {
            let o = sh + i * 40;
            sections.push((
                u32at(o + 12)?,
                u32at(o + 8)?,
                u32at(o + 20)?,
                u32at(o + 16)?,
            ));
        }
        Some(Pe {
            image_base,
            sections,
        })
    }

    pub(crate) fn file_offset(&self, va: u32) -> Option<usize> {
        let rva = (va as u64).checked_sub(self.image_base)? as u32;
        for &(sva, vsize, raw, rsize) in &self.sections {
            let size = vsize.max(rsize);
            if rva >= sva && rva < sva.saturating_add(size) {
                let off = rva - sva;
                if off >= rsize {
                    return None;
                }
                return Some((raw + off) as usize);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_and_interpolation() {
        let t = FwJpegTables::ijg_fallback();
        let q100 = t.qtables(100);
        assert!(q100.lum.iter().all(|&v| v == 1));
        let q50 = t.qtables(50);
        assert!(q50.lum[0] > 1);
        // Interpolation zwischen den Stützstellen ist monoton
        assert!(t.qtables(45).lum[10] >= t.qtables(55).lum[10]);
        let sp = t.selective_params(30, 90).unwrap();
        assert_eq!(sp.0, 90);
        assert!(t.selective_params(50, 50).is_none());
        assert!(FwJpegTables::from_json("{}").is_none());
        assert!(FwJpegTables::from_original_exe(b"MZ").is_none());
    }
}
