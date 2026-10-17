//! CMYK → RGB bitgenau wie das Original (Spezifikation 14 §1.6, `@00da27d0`; Referenz
//! `farbmodelle.py::cmyk_to_rgb_exakt`, Laufzeittest 4.096/4.096 Pixel).
//!
//! Ablauf:
//! 1. 8 → 16 Bit: `v16 = 257·v` ([`crate::models::to16`]).
//! 2. Je Achse Zelle und Rest im 9er-Gitter `0, 0x2000, …, 0xE000, 0xFFFF`
//!    (`0xFFFF` ⇒ Zelle 8, Rest 0; die letzte Zelle ist nur 0x1FFF breit, gewichtet wird trotzdem in 1/0x2000).
//! 3. 4D-multilinear in der Reihenfolge **C, M, Y, K**, nach **jedem** 1D-Schritt abgeschnitten:
//!    `L(a, b, w) = (a·(0x2000 − w) + b·w) >> 13`.
//! 4. 16 → 8 Bit: `v16·255/65535` abgeschnitten; Alpha = 0xFF.
//!
//! Die Gittertabelle (Ressource `CMYK/1001` der Installation des Originals, 6561 × 3 × u16) ist ein
//! Asset des Originals und wird **nicht** eingebettet: [`CmykTable::from_bytes`] lädt sie zur Laufzeit aus
//! einer vorhandenen Installation. Ohne Tabelle rechnet das Original `R = 255 − C` usw. (K ignoriert).

#![allow(clippy::needless_range_loop)] // Matrix-/Kanalindizes sind hier lesbarer

use crate::models::{to8, to16};
use crate::round::round_even;

/// Stützstellen je Achse (16 Bit).
pub const CMYK_GRID: [u32; 9] = [
    0, 0x2000, 0x4000, 0x6000, 0x8000, 0xA000, 0xC000, 0xE000, 0xFFFF,
];

/// Anzahl Gittereinträge (9⁴).
pub const CMYK_TABLE_ENTRIES: usize = 6561;

/// Größe der Ressource in Byte (6561 × 3 × u16).
pub const CMYK_TABLE_BYTES: usize = CMYK_TABLE_ENTRIES * 6;

/// Fehler beim Laden der CMYK-Tabelle.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CmykTableError {
    /// Die Daten sind kürzer als 39.366 Byte.
    #[error("{}", fp_i18n::t_args("err-color-cmyk-short", &[("n", fp_i18n::Value::from(.0.to_string())), ("expected", fp_i18n::Value::from(CMYK_TABLE_BYTES.to_string()))]))]
    TooShort(usize),
}

/// CMYK-Gittertabelle: Index `c·729 + m·81 + y·9 + k` (K läuft am schnellsten), je Eintrag R, G, B (16 Bit).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmykTable {
    entries: Vec<[u16; 3]>,
}

impl CmykTable {
    /// Liest die Ressource `CMYK/1001` (Little Endian, mindestens 39.366 Byte; Rest wird ignoriert).
    pub fn from_bytes(data: &[u8]) -> Result<CmykTable, CmykTableError> {
        if data.len() < CMYK_TABLE_BYTES {
            return Err(CmykTableError::TooShort(data.len()));
        }
        let entries = data[..CMYK_TABLE_BYTES]
            .as_chunks::<6>()
            .0
            .iter()
            .map(|c| {
                [
                    u16::from_le_bytes([c[0], c[1]]),
                    u16::from_le_bytes([c[2], c[3]]),
                    u16::from_le_bytes([c[4], c[5]]),
                ]
            })
            .collect();
        Ok(CmykTable { entries })
    }

    /// Tabelle aus Einträgen (genau 6561).
    pub fn from_entries(entries: Vec<[u16; 3]>) -> Option<CmykTable> {
        (entries.len() == CMYK_TABLE_ENTRIES).then_some(CmykTable { entries })
    }

    /// Eintrag am Gitterpunkt (c, m, y, k), je 0..=8.
    pub fn grid(&self, c: usize, m: usize, y: usize, k: usize) -> Option<[u16; 3]> {
        if c > 8 || m > 8 || y > 8 || k > 8 {
            return None;
        }
        self.entries.get(c * 729 + m * 81 + y * 9 + k).copied()
    }
}

/// 1D-Schritt `(a·(0x2000 − w) + b·w) >> 13` (`@00da2310`).
#[inline]
pub const fn lerp13(a: u32, b: u32, w: u32) -> u32 {
    (a * (0x2000 - w) + b * w) >> 13
}

/// Zelle und Rest einer 16-Bit-Koordinate.
#[inline]
fn cell(x: u32) -> (usize, u32) {
    let mut i = 8;
    for j in 1..9 {
        if x < CMYK_GRID[j] {
            i = j - 1;
            break;
        }
    }
    (i, x.saturating_sub(CMYK_GRID[i]))
}

/// Kern: CMYK (je 16 Bit) → RGB (je 16 Bit) per Kaskade C → M → Y → K.
pub fn cmyk16_to_rgb16(v: [u32; 4], table: &CmykTable) -> [u32; 3] {
    let (mut idx, mut fr) = ([0usize; 4], [0u32; 4]);
    for a in 0..4 {
        let (i, f) = cell(v[a].min(0xFFFF));
        idx[a] = i;
        fr[a] = f;
    }
    // Achse mit Rest 0: oberer Nachbar wird nie gebraucht (Gewicht 0) und nicht gelesen
    let grab = |dc: usize, dm: usize, dy: usize, dk: usize| -> [u32; 3] {
        if (dc == 1 && fr[0] == 0)
            || (dm == 1 && fr[1] == 0)
            || (dy == 1 && fr[2] == 0)
            || (dk == 1 && fr[3] == 0)
        {
            return [0; 3];
        }
        table
            .grid(idx[0] + dc, idx[1] + dm, idx[2] + dy, idx[3] + dk)
            .map(|e| [e[0] as u32, e[1] as u32, e[2] as u32])
            .unwrap_or([0; 3])
    };
    let mut corner = [[[[[0u32; 3]; 2]; 2]; 2]; 2];
    for (dc, a) in corner.iter_mut().enumerate() {
        for (dm, b) in a.iter_mut().enumerate() {
            for (dy, c) in b.iter_mut().enumerate() {
                for (dk, d) in c.iter_mut().enumerate() {
                    *d = grab(dc, dm, dy, dk);
                }
            }
        }
    }
    let mut out = [0u32; 3];
    for (ch, o) in out.iter_mut().enumerate() {
        let mut s1 = [[[0u32; 2]; 2]; 2];
        for dm in 0..2 {
            for dy in 0..2 {
                for dk in 0..2 {
                    s1[dm][dy][dk] =
                        lerp13(corner[0][dm][dy][dk][ch], corner[1][dm][dy][dk][ch], fr[0]);
                }
            }
        }
        let mut s2 = [[0u32; 2]; 2];
        for dy in 0..2 {
            for dk in 0..2 {
                s2[dy][dk] = lerp13(s1[0][dy][dk], s1[1][dy][dk], fr[1]);
            }
        }
        let s3 = [
            lerp13(s2[0][0], s2[1][0], fr[2]),
            lerp13(s2[0][1], s2[1][1], fr[2]),
        ];
        *o = lerp13(s3[0], s3[1], fr[3]);
    }
    out
}

/// CMYK-Bytes (255 = 100 %) → RGB wie das Original. Ohne Tabelle: `(255 − C, 255 − M, 255 − Y)`.
pub fn cmyk_to_rgb(c: u8, m: u8, y: u8, k: u8, table: Option<&CmykTable>) -> (u8, u8, u8) {
    let Some(table) = table else {
        return (255 - c, 255 - m, 255 - y);
    };
    let v = [
        to16(c) as u32,
        to16(m) as u32,
        to16(y) as u32,
        to16(k) as u32,
    ];
    let o = cmyk16_to_rgb16(v, table);
    let b = |x: u32| to8(x.min(0xFFFF) as u16);
    (b(o[0]), b(o[1]), b(o[2]))
}

/// ASE-CMYK-Wert (double 0..1) → Byte: `round_even(x·255)` auf ein Byte gekürzt (`@00672d60`).
pub fn ase_cmyk_byte(x: f64) -> u8 {
    if !x.is_finite() {
        return 0;
    }
    (round_even(x * 255.0) as i64 & 0xff) as u8
}

/// PostScript `setcmykcolor` (`@00a23da0`): `round_even(f·65535)`, davon das obere Byte.
pub fn postscript_cmyk_byte(f: f64) -> u8 {
    if !f.is_finite() {
        return 0;
    }
    ((round_even(f * 65535.0) as i64 & 0xFF00) >> 8) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_and_errors() {
        assert_eq!(cmyk_to_rgb(255, 0, 10, 200, None), (0, 255, 245));
        assert_eq!(
            CmykTable::from_bytes(&[0; 10]),
            Err(CmykTableError::TooShort(10))
        );
        assert_eq!(lerp13(0, 0x2000, 0x1000), 0x1000);
        assert_eq!(cell(0xFFFF), (8, 0));
        assert_eq!(cell(0xE001), (7, 1));
        assert_eq!(postscript_cmyk_byte(1.0), 255);
        assert_eq!(ase_cmyk_byte(0.5), 128);
    }

    #[test]
    fn synthetic_table_interpolates() {
        // Tabelle: R = 65535 − C-Anteil (linear), G = B = 0 → Interpolation muss linear sein
        let mut e = Vec::with_capacity(CMYK_TABLE_ENTRIES);
        for c in 0..9u32 {
            for _m in 0..9 {
                for _y in 0..9 {
                    for _k in 0..9 {
                        e.push([(0xFFFF - CMYK_GRID[c as usize]) as u16, 0, 0]);
                    }
                }
            }
        }
        let t = CmykTable::from_entries(e).unwrap();
        assert_eq!(cmyk_to_rgb(0, 0, 0, 0, Some(&t)), (255, 0, 0));
        assert_eq!(cmyk_to_rgb(255, 255, 255, 255, Some(&t)), (0, 0, 0));
        let (r, _, _) = cmyk_to_rgb(128, 0, 0, 0, Some(&t));
        assert!((126..=128).contains(&r), "{r}");
    }
}
