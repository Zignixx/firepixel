//! Farbfelddateien (Spezifikation 14 §2.2/§2.3, Referenz `re\farbe\farbfelddateien.py`).
//!
//! | Format | Lesen | Schreiben |
//! |---|---|---|
//! | ACT | Original-Art [`read_act_original`] (alle 3-Byte-Tripel, Anhang wird zur Farbe) und Photoshop-Art [`read_act_photoshop`] (768 Byte + Anzahl/Transparenzindex) | [`write_act_original`] (rohe Tripel, = `Color List.txt`), [`write_act_photoshop`] |
//! | ASE | [`read_ase`] (alle Blöcke), Übernahme wie das Original [`ase_to_original_swatches`] (nur RGB und CMYK) | [`write_ase_original`] (byte-genau wie das Original), [`write_ase`] (mit Namen/Gruppen) |
//! | ACO | [`read_aco`] (Version 1 und 2) | [`write_aco`] (Version 1 + 2) – das Original selbst kennt kein ACO |
//! | GIF | [`read_gif_palette`] (globale bzw. erste lokale Palette, Transparenzindex) | – |
//!
//! [`read_swatches_original`] liest wie das Farbfelder-Bedienfeld (Hinzufügen/Ersetzen): GIF-Palette,
//! bei Fehlschlag oder leerer Palette als ACT; ASE nach Signatur.

use crate::cmyk::{ase_cmyk_byte, cmyk_to_rgb, CmykTable};
use crate::round::round_even;
use crate::{Palette, Rgba8};

/// Meldung `key` mit einem Platzhalter in der aktuellen Sprache (messages.ftl).
fn m1(key: &str, a: &str, va: &dyn std::fmt::Display) -> String {
    fp_i18n::t_args(key, &[(a, fp_i18n::Value::from(va.to_string()))])
}

/// Fehler beim Lesen von Farbfelddateien.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SwatchError {
    /// Datei kürzer als nötig (ACT: < 3 Byte; das Original wirft dann eine Ausnahme).
    #[error("{}", m1("err-color-swatch-short", "n", .0))]
    TooShort(usize),
    /// Falsche Signatur.
    #[error("{}", m1("err-color-swatch-signature", "format", .0))]
    BadSignature(&'static str),
    /// Datei endet mitten in einem Block.
    #[error("{}", m1("err-color-swatch-truncated", "format", .0))]
    Truncated(&'static str),
    /// Datei ohne Palette.
    #[error("{}", fp_i18n::t("err-color-swatch-no-palette"))]
    NoPalette,
}

// ------------------------------------------------------------------ ACT

/// ACT wie das Original (`@00676270`): Datei < 3 Byte → Fehler; sonst **alle** vollständigen
/// 3-Byte-Tripel (R, G, B), ohne Obergrenze und ohne Auswertung des 4-Byte-Anhangs (eine
/// Photoshop-ACT mit 772 Byte ergibt 257 Farben). Alpha 0xFF.
pub fn read_act_original(data: &[u8]) -> Result<Vec<Rgba8>, SwatchError> {
    if data.len() < 3 {
        return Err(SwatchError::TooShort(data.len()));
    }
    Ok(data.as_chunks::<3>().0.iter().map(|c| Rgba8::rgb(c[0], c[1], c[2])).collect())
}

/// ACT wie das Original schreiben (`@00675030`): nur `Anzahl × (R, G, B)`, ohne Auffüllen und ohne
/// Anhang. Identisch mit dem Format von `Color List.txt` der Installation des Originals.
pub fn write_act_original(colors: &[Rgba8]) -> Vec<u8> {
    colors.iter().flat_map(|c| [c.r, c.g, c.b]).collect()
}

/// ACT nach Photoshop-Konvention: 768 Byte Palette, optional 4 Byte Anhang
/// (Anzahl u16 BE, 0 = 256; Transparenzindex u16 BE, 0xFFFF = keiner). Höchstens 256 Farben.
pub fn read_act_photoshop(data: &[u8]) -> Result<Palette, SwatchError> {
    if data.len() < 3 {
        return Err(SwatchError::TooShort(data.len()));
    }
    let mut n = 256usize;
    let mut transparent = None;
    if data.len() >= 772 {
        let cnt = u16::from_be_bytes([data[768], data[769]]) as usize;
        n = if cnt == 0 { 256 } else { cnt };
        let t = u16::from_be_bytes([data[770], data[771]]);
        if t != 0xFFFF {
            transparent = Some(t as usize);
        }
    }
    let n = n.min(data.len() / 3).min(256);
    let colors = data.as_chunks::<3>().0.iter().take(n).map(|c| Rgba8::rgb(c[0], c[1], c[2])).collect();
    Ok(Palette::with_transparent(colors, transparent))
}

/// ACT nach Photoshop-Konvention schreiben: 768 Byte (mit Schwarz aufgefüllt, höchstens 256 Farben)
/// plus Anhang (Anzahl, Transparenzindex bzw. 0xFFFF).
pub fn write_act_photoshop(p: &Palette) -> Vec<u8> {
    let n = p.colors.len().min(256);
    let mut out = Vec::with_capacity(772);
    for c in &p.colors[..n] {
        out.extend_from_slice(&[c.r, c.g, c.b]);
    }
    out.resize(768, 0);
    out.extend_from_slice(&(n as u16).to_be_bytes());
    let t = p.transparent_index.filter(|&i| i < n).map(|i| i as u16).unwrap_or(0xFFFF);
    out.extend_from_slice(&t.to_be_bytes());
    out
}

// ------------------------------------------------------------------ ASE

/// Farbmodell eines ASE-Eintrags.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AseModel {
    /// `RGB ` (3 Werte 0..1).
    Rgb,
    /// `CMYK` (4 Werte 0..1).
    Cmyk,
    /// `LAB ` (L 0..1 = 0..100 %, a, b in Einheiten).
    Lab,
    /// `Gray` (1 Wert).
    Gray,
    /// Unbekanntes Modell (vier ASCII-Zeichen).
    Other([u8; 4]),
}

impl AseModel {
    fn from_tag(t: [u8; 4]) -> AseModel {
        match &t {
            b"RGB " => AseModel::Rgb,
            b"CMYK" => AseModel::Cmyk,
            b"LAB " => AseModel::Lab,
            b"Gray" => AseModel::Gray,
            _ => AseModel::Other(t),
        }
    }

    fn tag(&self) -> [u8; 4] {
        match self {
            AseModel::Rgb => *b"RGB ",
            AseModel::Cmyk => *b"CMYK",
            AseModel::Lab => *b"LAB ",
            AseModel::Gray => *b"Gray",
            AseModel::Other(t) => *t,
        }
    }

    /// Anzahl der Werte.
    pub fn value_count(&self) -> usize {
        match self {
            AseModel::Rgb | AseModel::Lab => 3,
            AseModel::Cmyk => 4,
            AseModel::Gray => 1,
            AseModel::Other(_) => 0,
        }
    }
}

/// Ein Farbeintrag einer ASE-Datei.
#[derive(Debug, Clone, PartialEq)]
pub struct AseEntry {
    /// Name (ohne abschließendes NUL).
    pub name: String,
    pub model: AseModel,
    pub values: Vec<f32>,
    /// Farbtyp: 0 = Global, 1 = Spot, 2 = Normal (fehlt in kaputten Dateien).
    pub color_type: Option<u16>,
    /// Name der umgebenden Gruppe.
    pub group: Option<String>,
}

fn be_u16(d: &[u8], p: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*d.get(p)?, *d.get(p + 1)?]))
}

fn be_u32(d: &[u8], p: usize) -> Option<u32> {
    Some(u32::from_be_bytes([*d.get(p)?, *d.get(p + 1)?, *d.get(p + 2)?, *d.get(p + 3)?]))
}

fn utf16be_name(body: &[u8], nlen: usize) -> Option<String> {
    let bytes = body.get(2..2 + 2 * nlen)?;
    let units: Vec<u16> = bytes.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes(*c)).collect();
    Some(String::from_utf16_lossy(&units).trim_end_matches('\0').to_string())
}

/// Allgemeiner ASE-Leser: liefert alle Farbeinträge (auch Lab/Gray) mit Gruppenzuordnung.
/// Fehlende Bytes am Dateiende beenden das Lesen (wie die Referenz).
pub fn read_ase(data: &[u8]) -> Result<Vec<AseEntry>, SwatchError> {
    if data.len() < 12 {
        return if data.get(..4) == Some(b"ASEF") || data.len() < 4 {
            Err(SwatchError::TooShort(data.len()))
        } else {
            Err(SwatchError::BadSignature("ASE"))
        };
    }
    if &data[..4] != b"ASEF" {
        return Err(SwatchError::BadSignature("ASE"));
    }
    let nblocks = be_u32(data, 8).unwrap_or(0);
    let mut pos = 12usize;
    let mut out = Vec::new();
    let mut group: Option<String> = None;
    for _ in 0..nblocks {
        let (Some(btype), Some(blen)) = (be_u16(data, pos), be_u32(data, pos + 2)) else { break };
        let start = pos + 6;
        let end = start.saturating_add(blen as usize).min(data.len());
        let body = &data[start.min(data.len())..end];
        pos = start.saturating_add(blen as usize);
        if btype == 0xC002 {
            group = None;
            continue;
        }
        let Some(nlen) = be_u16(body, 0) else { continue };
        let nlen = nlen as usize;
        let name = utf16be_name(body, nlen).unwrap_or_default();
        let rest = body.get(2 + 2 * nlen..).unwrap_or(&[]);
        if btype == 0xC001 {
            group = Some(name);
            continue;
        }
        if btype != 0x0001 || rest.len() < 4 {
            continue;
        }
        let model = AseModel::from_tag([rest[0], rest[1], rest[2], rest[3]]);
        let n = model.value_count();
        if rest.len() < 4 + 4 * n {
            return Err(SwatchError::Truncated("ASE"));
        }
        let values = (0..n)
            .map(|i| f32::from_be_bytes([rest[4 + 4 * i], rest[5 + 4 * i], rest[6 + 4 * i], rest[7 + 4 * i]]))
            .collect();
        let color_type = be_u16(rest, 4 + 4 * n);
        out.push(AseEntry { name, model, values, color_type, group: group.clone() });
    }
    Ok(out)
}

/// Übernahme in die Farbfelder wie das Original (`@00672d60`): RGB → `round_even(v·255)`;
/// CMYK → Bytes `round_even(v·255)` → [`cmyk_to_rgb`] (ohne Tabelle `255 − x`); andere Modelle
/// werden übergangen.
pub fn ase_to_original_swatches(entries: &[AseEntry], cmyk_table: Option<&CmykTable>) -> Vec<Rgba8> {
    let byte = |v: f32| -> u8 {
        let x = v as f64 * 255.0;
        if x.is_finite() { (round_even(x) as i64 & 0xff) as u8 } else { 0 }
    };
    let mut out = Vec::new();
    for e in entries {
        match e.model {
            AseModel::Rgb if e.values.len() >= 3 => out.push(Rgba8::rgb(byte(e.values[0]), byte(e.values[1]), byte(e.values[2]))),
            AseModel::Cmyk if e.values.len() >= 4 => {
                let b = |i: usize| ase_cmyk_byte(e.values[i] as f64);
                let (r, g, bb) = cmyk_to_rgb(b(0), b(1), b(2), b(3), cmyk_table);
                out.push(Rgba8::rgb(r, g, bb));
            }
            _ => {}
        }
    }
    out
}

/// ASE byte-genau wie das Original schreiben (`@00675c70` + `ASE.dll@10002400`):
/// `"ASEF"`, Version 1.0, Blockanzahl = Farbanzahl (keine Gruppenblöcke); je Farbe Block 0x0001,
/// Länge 20, Namenslänge 0 (kein Name, kein NUL), `"RGB "`, drei `f32` BE = `x/255`, Farbtyp 2 (Normal).
/// Grenze des Originals von 3845 Farben (Puffer 100.000 Byte) wird nicht übernommen.
pub fn write_ase_original(colors: &[Rgba8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + 26 * colors.len());
    out.extend_from_slice(b"ASEF");
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&(colors.len() as u32).to_be_bytes());
    for c in colors {
        out.extend_from_slice(&1u16.to_be_bytes());
        out.extend_from_slice(&20u32.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes());
        out.extend_from_slice(b"RGB ");
        for v in [c.r, c.g, c.b] {
            out.extend_from_slice(&((v as f64 / 255.0) as f32).to_be_bytes());
        }
        out.extend_from_slice(&2u16.to_be_bytes());
    }
    out
}

fn push_name(out: &mut Vec<u8>, name: &str) {
    let units: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    out.extend_from_slice(&(units.len() as u16).to_be_bytes());
    for u in units {
        out.extend_from_slice(&u.to_be_bytes());
    }
}

/// Allgemeiner ASE-Schreiber: Namen mit NUL-Abschluss (robuster für fremde Leser), Gruppen als
/// Blöcke 0xC001/0xC002 um aufeinanderfolgende Einträge derselben Gruppe.
pub fn write_ase(entries: &[AseEntry]) -> Vec<u8> {
    let mut blocks: Vec<(u16, Vec<u8>)> = Vec::new();
    let mut current: Option<&str> = None;
    for e in entries {
        let g = e.group.as_deref();
        if g != current {
            if current.is_some() {
                blocks.push((0xC002, Vec::new()));
            }
            if let Some(name) = g {
                let mut b = Vec::new();
                push_name(&mut b, name);
                blocks.push((0xC001, b));
            }
            current = g;
        }
        let mut b = Vec::new();
        push_name(&mut b, &e.name);
        b.extend_from_slice(&e.model.tag());
        for i in 0..e.model.value_count() {
            b.extend_from_slice(&e.values.get(i).copied().unwrap_or(0.0).to_be_bytes());
        }
        b.extend_from_slice(&e.color_type.unwrap_or(2).to_be_bytes());
        blocks.push((0x0001, b));
    }
    if current.is_some() {
        blocks.push((0xC002, Vec::new()));
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"ASEF");
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&(blocks.len() as u32).to_be_bytes());
    for (t, b) in blocks {
        out.extend_from_slice(&t.to_be_bytes());
        out.extend_from_slice(&(b.len() as u32).to_be_bytes());
        out.extend_from_slice(&b);
    }
    out
}

// ------------------------------------------------------------------ ACO (Photoshop-Farbfelder)

/// Farbraum eines ACO-Eintrags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AcoSpace {
    /// 0: RGB, je 0..65535.
    Rgb,
    /// 1: HSB, Farbton 0..65535 (= 0..360°), S und B 0..65535.
    Hsb,
    /// 2: CMYK, je 0..65535 mit **0 = 100 % Farbe**.
    Cmyk,
    /// 7: Lab (L 0..10000, a/b vorzeichenbehaftet ×100).
    Lab,
    /// 8: Graustufen 0..10000.
    Gray,
    /// Andere Farbräume (Rohwert).
    Other(u16),
}

impl AcoSpace {
    fn from_u16(v: u16) -> AcoSpace {
        match v {
            0 => AcoSpace::Rgb,
            1 => AcoSpace::Hsb,
            2 => AcoSpace::Cmyk,
            7 => AcoSpace::Lab,
            8 => AcoSpace::Gray,
            o => AcoSpace::Other(o),
        }
    }

    fn to_u16(self) -> u16 {
        match self {
            AcoSpace::Rgb => 0,
            AcoSpace::Hsb => 1,
            AcoSpace::Cmyk => 2,
            AcoSpace::Lab => 7,
            AcoSpace::Gray => 8,
            AcoSpace::Other(o) => o,
        }
    }
}

/// Ein ACO-Eintrag (Rohwerte).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcoColor {
    pub space: AcoSpace,
    pub values: [u16; 4],
    /// Name (nur Version 2).
    pub name: Option<String>,
}

impl AcoColor {
    /// RGB-Eintrag aus einer Farbe (8 → 16 Bit mit ·257).
    pub fn from_rgb(c: Rgba8, name: Option<String>) -> AcoColor {
        AcoColor { space: AcoSpace::Rgb, values: [c.r as u16 * 257, c.g as u16 * 257, c.b as u16 * 257, 0], name }
    }

    /// Umrechnung nach RGB: RGB (obere 8 Bit gerundet), HSB (Gleitkomma-HSV), Graustufen
    /// (0 = Schwarz, A), CMYK über [`cmyk_to_rgb`]. Lab und andere → `None`.
    pub fn to_rgba8(&self, cmyk_table: Option<&CmykTable>) -> Option<Rgba8> {
        let v = self.values;
        let to8 = |x: u16| ((x as u32 * 255 + 32767) / 65535) as u8;
        match self.space {
            AcoSpace::Rgb => Some(Rgba8::rgb(to8(v[0]), to8(v[1]), to8(v[2]))),
            AcoSpace::Gray => {
                let g = ((v[0].min(10000) as u32 * 255 + 5000) / 10000) as u8;
                Some(Rgba8::rgb(g, g, g))
            }
            AcoSpace::Hsb => {
                let h = v[0] as f64 / 65535.0 * 6.0;
                let s = v[1] as f64 / 65535.0;
                let b = v[2] as f64 / 65535.0;
                let i = (h.floor() as i64).rem_euclid(6);
                let f = h - h.floor();
                let (p, q, t) = (b * (1.0 - s), b * (1.0 - s * f), b * (1.0 - s * (1.0 - f)));
                let (r, g, bb) = match i {
                    0 => (b, t, p),
                    1 => (q, b, p),
                    2 => (p, b, t),
                    3 => (p, q, b),
                    4 => (t, p, b),
                    _ => (b, p, q),
                };
                let c = |x: f64| (x * 255.0).round().clamp(0.0, 255.0) as u8;
                Some(Rgba8::rgb(c(r), c(g), c(bb)))
            }
            AcoSpace::Cmyk => {
                let b = |x: u16| 255 - (x >> 8) as u8;
                let (r, g, bb) = cmyk_to_rgb(b(v[0]), b(v[1]), b(v[2]), b(v[3]), cmyk_table);
                Some(Rgba8::rgb(r, g, bb))
            }
            AcoSpace::Lab | AcoSpace::Other(_) => None,
        }
    }
}

/// ACO lesen (Version 1; folgt ein Version-2-Abschnitt, werden dessen Einträge mit Namen geliefert).
pub fn read_aco(data: &[u8]) -> Result<Vec<AcoColor>, SwatchError> {
    if data.len() < 4 {
        return Err(SwatchError::TooShort(data.len()));
    }
    let read_section = |pos: usize, v2: bool| -> Result<(Vec<AcoColor>, usize), SwatchError> {
        let version = be_u16(data, pos).ok_or(SwatchError::Truncated("ACO"))?;
        if version != if v2 { 2 } else { 1 } {
            return Err(SwatchError::BadSignature("ACO"));
        }
        let count = be_u16(data, pos + 2).ok_or(SwatchError::Truncated("ACO"))? as usize;
        let mut p = pos + 4;
        let mut out = Vec::with_capacity(count.min(4096));
        for _ in 0..count {
            let sp = be_u16(data, p).ok_or(SwatchError::Truncated("ACO"))?;
            let mut values = [0u16; 4];
            for (i, v) in values.iter_mut().enumerate() {
                *v = be_u16(data, p + 2 + 2 * i).ok_or(SwatchError::Truncated("ACO"))?;
            }
            p += 10;
            let mut name = None;
            if v2 {
                let len = be_u32(data, p).ok_or(SwatchError::Truncated("ACO"))? as usize;
                p += 4;
                let bytes = data.get(p..p.saturating_add(len.saturating_mul(2))).ok_or(SwatchError::Truncated("ACO"))?;
                let units: Vec<u16> = bytes.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes(*c)).collect();
                name = Some(String::from_utf16_lossy(&units).trim_end_matches('\0').to_string());
                p += 2 * len;
            }
            out.push(AcoColor { space: AcoSpace::from_u16(sp), values, name });
        }
        Ok((out, p))
    };
    let first_version = be_u16(data, 0).unwrap_or(0);
    if first_version == 2 {
        return Ok(read_section(0, true)?.0);
    }
    let (v1, end) = read_section(0, false)?;
    if be_u16(data, end) == Some(2)
        && let Ok((v2, _)) = read_section(end, true)
        && v2.len() == v1.len()
    {
        return Ok(v2);
    }
    Ok(v1)
}

/// ACO schreiben: Abschnitt Version 1 und Version 2 (mit Namen; fehlende Namen leer).
pub fn write_aco(colors: &[AcoColor]) -> Vec<u8> {
    let n = colors.len().min(u16::MAX as usize);
    let mut out = Vec::new();
    for v2 in [false, true] {
        out.extend_from_slice(&(if v2 { 2u16 } else { 1u16 }).to_be_bytes());
        out.extend_from_slice(&(n as u16).to_be_bytes());
        for c in &colors[..n] {
            out.extend_from_slice(&c.space.to_u16().to_be_bytes());
            for v in c.values {
                out.extend_from_slice(&v.to_be_bytes());
            }
            if v2 {
                let units: Vec<u16> = c.name.as_deref().unwrap_or("").encode_utf16().chain(std::iter::once(0)).collect();
                out.extend_from_slice(&(units.len() as u32).to_be_bytes());
                for u in units {
                    out.extend_from_slice(&u.to_be_bytes());
                }
            }
        }
    }
    out
}

// ------------------------------------------------------------------ GIF-Palette

/// Palette einer GIF-Datei: globale Farbtabelle, sonst die lokale Tabelle des ersten Bilds;
/// Transparenzindex aus der ersten Grafiksteuerungs-Erweiterung vor dem ersten Bild.
/// (Umgang des Originals mit lokalen Paletten ist offen, 14 offene Frage 13.)
pub fn read_gif_palette(data: &[u8]) -> Result<Palette, SwatchError> {
    if data.len() < 13 {
        return Err(SwatchError::TooShort(data.len()));
    }
    if &data[..3] != b"GIF" {
        return Err(SwatchError::BadSignature("GIF"));
    }
    let table = |pos: usize, packed: u8| -> Option<(Vec<Rgba8>, usize)> {
        let n = 1usize << ((packed & 7) + 1);
        let bytes = data.get(pos..pos + 3 * n)?;
        Some((bytes.as_chunks::<3>().0.iter().map(|c| Rgba8::rgb(c[0], c[1], c[2])).collect(), pos + 3 * n))
    };
    let packed = data[10];
    let mut pos = 13;
    let mut global = None;
    if packed & 0x80 != 0 {
        let (t, p) = table(pos, packed).ok_or(SwatchError::Truncated("GIF"))?;
        global = Some(t);
        pos = p;
    }
    let mut transparent = None;
    let mut local = None;
    // Blöcke bis zum ersten Bild durchgehen
    while let Some(&b) = data.get(pos) {
        match b {
            0x21 => {
                let label = *data.get(pos + 1).ok_or(SwatchError::Truncated("GIF"))?;
                let mut p = pos + 2;
                if label == 0xF9 && data.get(p) == Some(&4) {
                    let flags = *data.get(p + 1).ok_or(SwatchError::Truncated("GIF"))?;
                    let idx = *data.get(p + 4).ok_or(SwatchError::Truncated("GIF"))?;
                    if flags & 1 != 0 && transparent.is_none() {
                        transparent = Some(idx as usize);
                    }
                }
                // Unterblöcke überspringen
                loop {
                    let len = *data.get(p).ok_or(SwatchError::Truncated("GIF"))? as usize;
                    p += 1 + len;
                    if len == 0 {
                        break;
                    }
                }
                pos = p;
            }
            0x2C => {
                let ipacked = *data.get(pos + 9).ok_or(SwatchError::Truncated("GIF"))?;
                if global.is_none() && ipacked & 0x80 != 0 {
                    local = table(pos + 10, ipacked).map(|t| t.0);
                }
                break;
            }
            _ => break,
        }
    }
    let colors = global.or(local).ok_or(SwatchError::NoPalette)?;
    Ok(Palette::with_transparent(colors, transparent))
}

/// Farbfelddatei lesen wie das Farbfelder-Bedienfeld (Hinzufügen/Ersetzen): `ASEF` → ASE
/// (Übernahme wie das Original); `GIF` → Palette, bei Fehlschlag oder leerer Palette als ACT;
/// sonst ACT (Original-Art).
pub fn read_swatches_original(data: &[u8], cmyk_table: Option<&CmykTable>) -> Result<Vec<Rgba8>, SwatchError> {
    if data.starts_with(b"ASEF") {
        return Ok(ase_to_original_swatches(&read_ase(data)?, cmyk_table));
    }
    if data.starts_with(b"GIF")
        && let Ok(p) = read_gif_palette(data)
        && !p.colors.is_empty()
    {
        return Ok(p.colors);
    }
    read_act_original(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn act_conventions() {
        let mut ps: Vec<u8> = (0u8..48).collect();
        ps.resize(768, 0);
        ps.extend_from_slice(&[0, 16, 0xFF, 0xFF]);
        let fw = read_act_original(&ps).unwrap();
        assert_eq!(fw.len(), 257);
        assert_eq!(fw[256], Rgba8::rgb(0, 16, 255));
        let p = read_act_photoshop(&ps).unwrap();
        assert_eq!(p.colors.len(), 16);
        assert_eq!(p.transparent_index, None);
        assert_eq!(write_act_photoshop(&p), ps);
        assert_eq!(read_act_original(&[1, 2]), Err(SwatchError::TooShort(2)));
        assert_eq!(write_act_original(&fw[..2]), vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn ase_roundtrip() {
        let cols: Vec<Rgba8> = (0..=255u8).map(|v| Rgba8::rgb(v, 255 - v, v / 2)).collect();
        let data = write_ase_original(&cols);
        assert_eq!(data.len(), 12 + 26 * 256);
        let back = ase_to_original_swatches(&read_ase(&data).unwrap(), None);
        assert_eq!(back, cols);
        let entries = vec![
            AseEntry { name: "Rot".into(), model: AseModel::Rgb, values: vec![1.0, 0.0, 0.0], color_type: Some(2), group: Some("G".into()) },
            AseEntry { name: "C".into(), model: AseModel::Cmyk, values: vec![1.0, 0.0, 0.0, 0.0], color_type: Some(0), group: None },
            AseEntry { name: "L".into(), model: AseModel::Lab, values: vec![0.5, 1.0, -1.0], color_type: Some(1), group: None },
        ];
        let rd = read_ase(&write_ase(&entries)).unwrap();
        assert_eq!(rd, entries);
        assert_eq!(ase_to_original_swatches(&rd, None), vec![Rgba8::rgb(255, 0, 0), Rgba8::rgb(0, 255, 255)]);
        assert_eq!(read_ase(b"XXXX00000000"), Err(SwatchError::BadSignature("ASE")));
    }

    #[test]
    fn aco_roundtrip() {
        let cols = vec![
            AcoColor::from_rgb(Rgba8::rgb(255, 128, 0), Some("Orange".into())),
            AcoColor { space: AcoSpace::Gray, values: [5000, 0, 0, 0], name: Some(String::new()) },
            AcoColor { space: AcoSpace::Hsb, values: [0, 65535, 65535, 0], name: Some("Rot".into()) },
        ];
        let data = write_aco(&cols);
        let back = read_aco(&data).unwrap();
        assert_eq!(back, cols);
        assert_eq!(back[0].to_rgba8(None), Some(Rgba8::rgb(255, 128, 0)));
        assert_eq!(back[1].to_rgba8(None), Some(Rgba8::rgb(128, 128, 128)));
        assert_eq!(back[2].to_rgba8(None), Some(Rgba8::rgb(255, 0, 0)));
        assert!(read_aco(&data[..7]).is_err());
    }

    #[test]
    fn gif_palette() {
        // minimales GIF89a mit globaler 2-Farben-Tabelle und Transparenz-GCE
        let mut g = b"GIF89a".to_vec();
        g.extend_from_slice(&[1, 0, 1, 0, 0x80, 0, 0]);
        g.extend_from_slice(&[255, 0, 0, 0, 0, 255]);
        g.extend_from_slice(&[0x21, 0xF9, 4, 1, 0, 0, 1, 0]);
        g.extend_from_slice(&[0x2C, 0, 0, 0, 0, 1, 0, 1, 0, 0, 2, 2, 0x4C, 0x01, 0, 0x3B]);
        let p = read_gif_palette(&g).unwrap();
        assert_eq!(p.colors, vec![Rgba8::rgb(255, 0, 0), Rgba8::rgb(0, 0, 255)]);
        assert_eq!(p.transparent_index, Some(1));
        assert_eq!(read_swatches_original(&g, None).unwrap().len(), 2);
        assert!(read_gif_palette(&g[..15]).is_err());
    }
}
