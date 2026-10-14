//! PSD/PSB-Strukturleser (12 §3; Referenz `re\import\psd.py`): Kopf, Bildressourcen (Auflösung 1005,
//! globaler Winkel 1037, Hilfslinien 1032), Ebenendatensätze mit Masken und Zusatzblöcken (`luni`,
//! `lsct`, `lfx2`/`lmfx`/`lrFX`, `iOpa`, `SoCo`/`GdFl`/`PtFl`, `vmsk`/`vsms`, `TySh`, Einstellungen,
//! Smart Objects), Kanäle (roh, PackBits, ZIP mit/ohne Prädiktion; 8/16/32 Bit → 8 Bit), Composite.

use super::descriptor::{descriptor, Descriptor, Reader};
use crate::{ImportError, Result};

/// Photoshop-Mischmodus-Schlüssel → String-ID (wie PSL).
pub fn blend_name(key: &str) -> &'static str {
    match key {
        "pass" => "passThrough",
        "norm" => "normal",
        "diss" => "dissolve",
        "dark" => "darken",
        "mul " => "multiply",
        "idiv" => "colorBurn",
        "lbrn" => "linearBurn",
        "dkCl" => "darkerColor",
        "lite" => "lighten",
        "scrn" => "screen",
        "div " => "colorDodge",
        "lddg" => "linearDodge",
        "lgCl" => "lighterColor",
        "over" => "overlay",
        "sLit" => "softLight",
        "hLit" => "hardLight",
        "vLit" => "vividLight",
        "lLit" => "linearLight",
        "pLit" => "pinLight",
        "hMix" => "hardMix",
        "diff" => "difference",
        "smud" => "exclusion",
        "fsub" => "subtract",
        "fdiv" => "divide",
        "hue " => "hue",
        "sat " => "saturation",
        "colr" => "color",
        "lum " => "luminosity",
        _ => "normal",
    }
}

/// Einstellungsebenen-Schlüssel.
pub const ADJUSTMENT_KEYS: [&str; 17] = ["levl", "curv", "brit", "blnc", "hue ", "hue2", "selc", "thrs", "nvrt", "post", "mixr", "grdm", "phfl", "expA", "vibA", "blwh", "clrL"];

/// Ebenenart wie das Original (PSL-Bibliothek) sie unterscheidet (12 §3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerKind {
    /// Gruppe (offen/geschlossen).
    Section { open: bool },
    /// Abschlussmarke einer Gruppe (`lsct` 3).
    SectionDivider,
    Text,
    /// Füllebene (`SoCo`/`GdFl`/`PtFl`).
    Content,
    Adjustment,
    Background,
    SmartObject,
    Pixel,
}

/// Benutzer-/Ebenenmaske.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerMask {
    /// Rechteck (links, oben, rechts, unten).
    pub rect: [i32; 4],
    pub default_color: u8,
    pub flags: u8,
    /// Maskenpixel (8 Bit, `rect`).
    pub pixels: Vec<u8>,
}

impl LayerMask {
    pub fn disabled(&self) -> bool {
        self.flags & 2 != 0
    }
}

/// Ein Pfadknoten einer Vektormaske in Pixeln: (Eingang, Anker, Ausgang), je (x, y).
pub type PathKnot = [(f64, f64); 3];

/// Unterpfad einer Vektormaske.
#[derive(Clone, Debug, PartialEq)]
pub struct SubPath {
    pub closed: bool,
    pub knots: Vec<PathKnot>,
}

/// Eine Ebene (Datei-Reihenfolge unten → oben).
#[derive(Clone, Debug, PartialEq)]
pub struct PsdLayer {
    pub name: String,
    /// Rechteck (links, oben, rechts, unten).
    pub rect: [i32; 4],
    /// Blend-Key als String-ID.
    pub blend: &'static str,
    pub opacity: u8,
    pub clipping: bool,
    pub visible: bool,
    /// Pixel RGBA (`rect`), aus den Kanälen 0/1/2/−1.
    pub rgba: Vec<u8>,
    pub mask: Option<LayerMask>,
    /// Gruppe: `lsct`-Typ (1 offen, 2 zu, 3 Abschluss) und ggf. Gruppen-Mischmodus.
    pub section: Option<(u32, Option<&'static str>)>,
    /// Ebeneneffekte (`lfx2`/`lmfx`).
    pub effects: Option<Descriptor>,
    /// Fülldeckkraft (`iOpa`), wird nicht ausgewertet (12 §3.5).
    pub fill_opacity: Option<u8>,
    /// Füllebene: (Schlüssel, Deskriptor).
    pub fill: Option<(String, Descriptor)>,
    /// Vektormaske.
    pub vector_mask: Option<(Vec<SubPath>, u32)>,
    /// Text: (Transformation xx xy yx yy tx ty, Text, Textdeskriptor, EngineData-Rohtext).
    pub text: Option<([f64; 6], String, Option<Descriptor>, String)>,
    /// Einstellung: (Schlüssel, Rohdaten).
    pub adjustment: Option<(String, Vec<u8>)>,
    pub smart_object: bool,
    pub is_background: bool,
    /// Alle Zusatzblock-Schlüssel (Diagnose).
    pub block_keys: Vec<String>,
}

impl PsdLayer {
    /// Ebenenart.
    pub fn kind(&self) -> LayerKind {
        if let Some((t, _)) = self.section {
            if t == 1 || t == 2 {
                return LayerKind::Section { open: t == 1 };
            }
            if t == 3 {
                return LayerKind::SectionDivider;
            }
        }
        if self.text.is_some() {
            return LayerKind::Text;
        }
        if self.fill.is_some() {
            return LayerKind::Content;
        }
        if self.adjustment.is_some() {
            return LayerKind::Adjustment;
        }
        if self.is_background {
            return LayerKind::Background;
        }
        if self.smart_object {
            return LayerKind::SmartObject;
        }
        LayerKind::Pixel
    }

    pub fn width(&self) -> usize {
        (self.rect[2] - self.rect[0]).max(0) as usize
    }
    pub fn height(&self) -> usize {
        (self.rect[3] - self.rect[1]).max(0) as usize
    }
}

/// Bildressourcen-Auswertung.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ResourceInfo {
    pub dpi: Option<f32>,
    /// Globaler Lichtwinkel (1037).
    pub global_angle: Option<f64>,
    /// Hilfslinien (Position px, vertikal?).
    pub guides: Vec<(f32, bool)>,
}

/// Gelesene PSD-Datei.
#[derive(Clone, Debug, PartialEq)]
pub struct PsdFile {
    pub psb: bool,
    pub width: u32,
    pub height: u32,
    pub channels: u16,
    pub depth: u16,
    /// Farbmodus (0 Bitmap, 1 Grau, 2 indiziert, 3 RGB, 4 CMYK, 7 Mehrkanal, 8 Duplex, 9 Lab).
    pub mode: u16,
    pub resources: ResourceInfo,
    pub layers: Vec<PsdLayer>,
    /// Composite RGBA (`width·height`); Farben in der Datei gegen Weiß mattiert.
    pub composite: Vec<u8>,
    /// Farbtabelle bei indizierten Dateien.
    pub color_table: Vec<[u8; 3]>,
}

fn bad(s: &str) -> ImportError {
    ImportError::Corrupt(format!("PSD: {}", crate::part(s)))
}

fn packbits(data: &[u8], expected: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(expected);
    let mut i = 0;
    while i < data.len() && out.len() < expected {
        let c = data[i];
        i += 1;
        if c < 128 {
            let n = c as usize + 1;
            let end = (i + n).min(data.len());
            out.extend_from_slice(&data[i..end]);
            i += n;
        } else if c > 128 {
            if let Some(&v) = data.get(i) {
                out.extend(std::iter::repeat_n(v, 257 - c as usize));
            }
            i += 1;
        }
    }
    out.resize(expected, 0);
    out
}

fn to8(raw: &[u8], depth: u16, n: usize) -> Vec<u8> {
    match depth {
        16 => (0..n).map(|i| raw.get(i * 2).copied().unwrap_or(0)).collect(),
        32 => (0..n)
            .map(|i| {
                let b = raw.get(i * 4..i * 4 + 4).map(|v| f32::from_be_bytes([v[0], v[1], v[2], v[3]])).unwrap_or(0.0);
                (b.clamp(0.0, 1.0) * 255.0).round() as u8
            })
            .collect(),
        1 => (0..n).map(|i| if raw.get(i / 8).is_some_and(|b| b & (0x80 >> (i % 8)) != 0) { 0 } else { 255 }).collect(),
        _ => {
            let mut v = raw[..raw.len().min(n)].to_vec();
            v.resize(n, 0);
            v
        }
    }
}

/// Kanaldaten (Kompression + Daten) für `w × h` lesen und auf 8 Bit bringen.
fn channel(r: &mut Reader, w: usize, h: usize, depth: u16, len: usize, psb: bool) -> Option<Vec<u8>> {
    let start = r.o;
    let end = start.checked_add(len)?;
    if len < 2 {
        r.o = end.min(r.end);
        return Some(vec![0; w * h]);
    }
    let comp = r.u16()?;
    let rowbytes = (w * depth.max(1) as usize).div_ceil(8);
    let size = rowbytes.checked_mul(h)?;
    if size as u64 > crate::MAX_PIXELS * 4 {
        return None;
    }
    let raw: Vec<u8> = if w == 0 || h == 0 {
        Vec::new()
    } else {
        match comp {
            0 => r.take(size.min(end.saturating_sub(r.o)))?.to_vec(),
            1 => {
                let mut counts = Vec::with_capacity(h);
                for _ in 0..h {
                    counts.push(if psb { r.u32()? as usize } else { r.u16()? as usize });
                }
                let mut out = Vec::with_capacity(size);
                for c in counts {
                    let d = r.take(c.min(end.saturating_sub(r.o)))?;
                    out.extend(packbits(d, rowbytes));
                }
                out
            }
            2 | 3 => {
                let d = r.take(end.saturating_sub(r.o))?;
                let mut dec = flate2::read::ZlibDecoder::new(d);
                let mut raw = Vec::with_capacity(size);
                use std::io::Read;
                let _ = dec.by_ref().take(size as u64).read_to_end(&mut raw);
                raw.resize(size, 0);
                if comp == 3 {
                    match depth {
                        8 => {
                            for y in 0..h {
                                for x in 1..rowbytes {
                                    raw[y * rowbytes + x] = raw[y * rowbytes + x].wrapping_add(raw[y * rowbytes + x - 1]);
                                }
                            }
                        }
                        16 => {
                            for y in 0..h {
                                for x in 1..w {
                                    let o = y * rowbytes + x * 2;
                                    let prev = u16::from_be_bytes([raw[o - 2], raw[o - 1]]);
                                    let cur = u16::from_be_bytes([raw[o], raw[o + 1]]);
                                    let v = cur.wrapping_add(prev).to_be_bytes();
                                    raw[o] = v[0];
                                    raw[o + 1] = v[1];
                                }
                            }
                        }
                        _ => {}
                    }
                }
                raw
            }
            _ => vec![0; size],
        }
    };
    r.o = end.min(r.end);
    Some(to8(&raw, depth, w * h))
}

fn parse_resources(r: &mut Reader, end: usize) -> ResourceInfo {
    let mut info = ResourceInfo::default();
    while r.o + 12 <= end {
        let Some(sig) = r.key4() else { break };
        if sig != "8BIM" {
            break;
        }
        let (Some(id), Some(nl)) = (r.u16(), r.u8()) else { break };
        if r.take(nl as usize).is_none() {
            break;
        }
        if (nl as usize + 1) % 2 == 1 {
            r.take(1);
        }
        let Some(n) = r.u32() else { break };
        let n = n as usize;
        let Some(data) = r.take(n) else { break };
        if n % 2 == 1 {
            r.take(1);
        }
        let mut d = Reader::new(data);
        match id {
            1005 => {
                if let Some(hres) = d.u32() {
                    let unit = d.u16().unwrap_or(1);
                    let v = hres as f32 / 65536.0;
                    info.dpi = Some(if unit == 2 { v * 2.54 } else { v });
                }
            }
            1037 => info.global_angle = d.i32().map(|v| v as f64),
            1032 => {
                let _ = (d.u32(), d.u32(), d.u32());
                if let Some(cnt) = d.u32() {
                    for _ in 0..cnt.min(10_000) {
                        let (Some(loc), Some(dir)) = (d.i32(), d.u8()) else { break };
                        info.guides.push((loc as f32 / 32.0, dir == 0));
                    }
                }
            }
            _ => {}
        }
    }
    info
}

fn parse_path_records(data: &[u8], w: f64, h: f64) -> Vec<SubPath> {
    let mut out: Vec<SubPath> = Vec::new();
    let fix = |b: &[u8]| -> f64 { i32::from_be_bytes([b[0], b[1], b[2], b[3]]) as f64 / (1 << 24) as f64 };
    for rec in data.as_chunks::<26>().0 {
        let sel = u16::from_be_bytes([rec[0], rec[1]]);
        match sel {
            0 | 3 => out.push(SubPath { closed: sel == 0, knots: Vec::new() }),
            1 | 2 | 4 | 5 => {
                let pt = |o: usize| (fix(&rec[o + 4..o + 8]) * w, fix(&rec[o..o + 4]) * h);
                let k = [pt(2), pt(10), pt(18)];
                if let Some(sp) = out.last_mut() {
                    sp.knots.push(k);
                }
            }
            _ => {}
        }
    }
    out.retain(|s| !s.knots.is_empty());
    out
}

fn read_blocks<'a>(r: &mut Reader<'a>, end: usize, psb: bool, pad: usize) -> Vec<(String, &'a [u8])> {
    const LONG: [&str; 13] = ["LMsk", "Lr16", "Lr32", "Layr", "Mt16", "Mt32", "Mtrn", "Alph", "FMsk", "lnk2", "FEid", "FXid", "PxSD"];
    let mut out = Vec::new();
    while r.o + 12 <= end {
        let save = r.o;
        let Some(sig) = r.key4() else { break };
        if sig != "8BIM" && sig != "8B64" {
            r.o = save;
            break;
        }
        let Some(k) = r.key4() else { break };
        let n = if psb && LONG.contains(&k.as_str()) { r.u64().map(|v| v as usize) } else { r.u32().map(|v| v as usize) };
        let Some(n) = n else { break };
        let Some(d) = r.take(n.min(end.saturating_sub(r.o))) else { break };
        out.push((k, d));
        if pad > 1 && n % pad != 0 {
            r.take(pad - n % pad);
        }
    }
    out
}

fn ustr_block(d: &[u8]) -> Option<String> {
    let mut r = Reader::new(d);
    let n = r.u32()? as usize;
    let raw = r.take(n.checked_mul(2)?)?;
    let u: Vec<u16> = raw.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
    Some(String::from_utf16_lossy(&u).trim_end_matches('\0').to_string())
}

/// PSD/PSB lesen.
pub fn read(b: &[u8]) -> Result<PsdFile> {
    let mut r = Reader::new(b);
    if r.take(4) != Some(b"8BPS") {
        return Err(bad(&fp_i18n::t("err-import-signature-missing")));
    }
    let ver = r.u16().ok_or_else(|| bad("err-import-part-header"))?;
    let psb = ver == 2;
    r.take(6).ok_or_else(|| bad("err-import-part-header"))?;
    let ch = r.u16().ok_or_else(|| bad("err-import-part-header"))?;
    let h = r.u32().ok_or_else(|| bad("err-import-part-header"))?;
    let w = r.u32().ok_or_else(|| bad("err-import-part-header"))?;
    let depth = r.u16().ok_or_else(|| bad("err-import-part-header"))?;
    let mode = r.u16().ok_or_else(|| bad("err-import-part-header"))?;
    crate::check_size(w, h)?;
    let n = r.u32().ok_or_else(|| bad("err-import-part-color-mode-data"))? as usize;
    let cmd = r.take(n).ok_or_else(|| bad("err-import-part-color-mode-data"))?;
    let color_table: Vec<[u8; 3]> = if mode == 2 && cmd.len() >= 768 { (0..256).map(|i| [cmd[i], cmd[256 + i], cmd[512 + i]]).collect() } else { Vec::new() };
    let n = r.u32().ok_or_else(|| bad("err-import-part-resources"))? as usize;
    let res_end = r.o.checked_add(n).ok_or_else(|| bad("err-import-part-resources"))?;
    let resources = parse_resources(&mut Reader::sub(b, r.o, res_end), res_end);
    r.o = res_end.min(b.len());
    let lm_len = if psb { r.u64().map(|v| v as usize) } else { r.u32().map(|v| v as usize) }.ok_or_else(|| bad("err-import-part-layer-section"))?;
    let lm_end = r.o.saturating_add(lm_len).min(b.len());
    let mut layers: Vec<PsdLayer> = Vec::new();
    if lm_len > 0 {
        let li_len = if psb { r.u64().map(|v| v as usize) } else { r.u32().map(|v| v as usize) }.unwrap_or(0);
        let li_end = r.o.saturating_add(li_len).min(b.len());
        if li_len > 0 {
            let cnt = r.i16().ok_or_else(|| bad("err-import-part-layer-count"))?.unsigned_abs() as usize;
            let mut chans: Vec<Vec<(i16, usize)>> = Vec::new();
            for _ in 0..cnt.min(16_000) {
                let rect = [0; 4].map(|_: i32| 0);
                let (t, l, bt, rt) = (r.i32().ok_or_else(|| bad("err-import-part-layer"))?, r.i32().ok_or_else(|| bad("err-import-part-layer"))?, r.i32().ok_or_else(|| bad("err-import-part-layer"))?, r.i32().ok_or_else(|| bad("err-import-part-layer"))?);
                let _ = rect;
                let nch = r.u16().ok_or_else(|| bad("err-import-part-layer"))? as usize;
                let mut cl = Vec::with_capacity(nch);
                for _ in 0..nch.min(64) {
                    let id = r.i16().ok_or_else(|| bad("err-import-part-channel"))?;
                    let len = if psb { r.u64().map(|v| v as usize) } else { r.u32().map(|v| v as usize) }.ok_or_else(|| bad("err-import-part-channel"))?;
                    cl.push((id, len));
                }
                r.key4().ok_or_else(|| bad("err-import-part-layer"))?;
                let bkey = r.key4().ok_or_else(|| bad("err-import-part-layer"))?;
                let opacity = r.u8().ok_or_else(|| bad("err-import-part-layer"))?;
                let clipping = r.u8().ok_or_else(|| bad("err-import-part-layer"))? != 0;
                let flags = r.u8().ok_or_else(|| bad("err-import-part-layer"))?;
                r.u8();
                let elen = r.u32().ok_or_else(|| bad("err-import-part-layer"))? as usize;
                let eend = r.o.saturating_add(elen).min(li_end);
                let ml = r.u32().ok_or_else(|| bad("err-import-part-mask"))? as usize;
                let mdata = r.take(ml).ok_or_else(|| bad("err-import-part-mask"))?;
                let mask = if ml >= 18 {
                    let mut m = Reader::new(mdata);
                    let (mt, mlft, mb, mr) = (m.i32().unwrap_or(0), m.i32().unwrap_or(0), m.i32().unwrap_or(0), m.i32().unwrap_or(0));
                    Some(LayerMask { rect: [mlft, mt, mr, mb], default_color: m.u8().unwrap_or(0), flags: m.u8().unwrap_or(0), pixels: Vec::new() })
                } else {
                    None
                };
                let bl = r.u32().ok_or_else(|| bad("err-import-part-blending-ranges"))? as usize;
                r.take(bl).ok_or_else(|| bad("err-import-part-blending-ranges"))?;
                let nl = r.u8().ok_or_else(|| bad("err-import-part-name"))? as usize;
                let pname: String = r.take(nl).ok_or_else(|| bad("err-import-part-name"))?.iter().map(|&c| c as char).collect();
                let pad = (4 - (nl + 1) % 4) % 4;
                r.take(pad);
                let blocks = read_blocks(&mut r, eend, psb, 1);
                r.o = eend;
                let mut l = PsdLayer {
                    name: pname,
                    rect: [l, t, rt, bt],
                    blend: blend_name(&bkey),
                    opacity,
                    clipping,
                    visible: flags & 2 == 0,
                    rgba: Vec::new(),
                    mask,
                    section: None,
                    effects: None,
                    fill_opacity: None,
                    fill: None,
                    vector_mask: None,
                    text: None,
                    adjustment: None,
                    smart_object: false,
                    is_background: false,
                    block_keys: blocks.iter().map(|(k, _)| k.clone()).collect(),
                };
                for (k, d) in &blocks {
                    match k.as_str() {
                        "luni" => {
                            if let Some(s) = ustr_block(d) {
                                l.name = s;
                            }
                        }
                        "lsct" => {
                            let mut br = Reader::new(d);
                            if let Some(t) = br.u32() {
                                let bm = if br.left() >= 8 {
                                    br.take(4);
                                    br.key4().map(|k| blend_name(&k))
                                } else {
                                    None
                                };
                                l.section = Some((t, bm));
                            }
                        }
                        "lfx2" | "lmfx" => {
                            let mut br = Reader::new(d);
                            let _ = (br.u32(), br.u32());
                            if let Some(desc) = descriptor(&mut br)
                                && (l.effects.is_none() || k == "lfx2")
                            {
                                l.effects = Some(desc);
                            }
                        }
                        "iOpa" => l.fill_opacity = d.first().copied(),
                        "SoCo" | "GdFl" | "PtFl" => {
                            let mut br = Reader::new(d);
                            br.u32();
                            if let Some(desc) = descriptor(&mut br) {
                                l.fill = Some((k.clone(), desc));
                            } else {
                                l.fill = Some((k.clone(), Descriptor::default()));
                            }
                        }
                        "vmsk" | "vsms" => {
                            let mut br = Reader::new(d);
                            let _ = br.u32();
                            let fl = br.u32().unwrap_or(0);
                            let rest = &d[d.len().min(8)..];
                            l.vector_mask = Some((parse_path_records(rest, w as f64, h as f64), fl));
                        }
                        "TySh" => {
                            let mut br = Reader::new(d);
                            let _ = br.u16();
                            let mut m = [0f64; 6];
                            for v in m.iter_mut() {
                                *v = br.f64().unwrap_or(0.0);
                            }
                            let _ = (br.u16(), br.u32());
                            let desc = descriptor(&mut br);
                            let text = desc.as_ref().and_then(|x| x.str("text").or_else(|| x.str("Txt ")).map(str::to_string)).unwrap_or_default();
                            // Latin-1 (ein Zeichen je Byte), damit UTF-16-Zeichenketten `(\xfe\xff…)` lesbar bleiben
                            let engine: String = d.iter().map(|&b| b as char).collect();
                            l.text = Some((m, text, desc, engine));
                        }
                        "SoLd" | "PlLd" | "SoLE" => l.smart_object = true,
                        k2 if ADJUSTMENT_KEYS.contains(&k2) => l.adjustment = Some((k.clone(), d.to_vec())),
                        _ => {}
                    }
                }
                chans.push(cl);
                layers.push(l);
            }
            // Kanaldaten
            for (l, cl) in layers.iter_mut().zip(&chans) {
                let (lw, lh) = (l.width(), l.height());
                let mut planes: [Option<Vec<u8>>; 4] = [None, None, None, None];
                for &(id, len) in cl {
                    let (cw, chh) = match (id, &l.mask) {
                        (-2 | -3, Some(m)) => ((m.rect[2] - m.rect[0]).max(0) as usize, (m.rect[3] - m.rect[1]).max(0) as usize),
                        _ => (lw, lh),
                    };
                    let data = channel(&mut r, cw, chh, depth, len, psb).ok_or_else(|| bad("err-import-part-channel-data"))?;
                    match id {
                        0..=2 => planes[id as usize] = Some(data),
                        -1 => planes[3] = Some(data),
                        -2 => {
                            if let Some(m) = l.mask.as_mut() {
                                m.pixels = data;
                            }
                        }
                        _ => {}
                    }
                }
                let n = lw * lh;
                let get = |p: &Option<Vec<u8>>, i: usize, d: u8| p.as_ref().and_then(|v| v.get(i)).copied().unwrap_or(d);
                let gray = mode == 1 || mode == 8;
                let mut rgba = Vec::with_capacity(n * 4);
                for i in 0..n {
                    let rr = get(&planes[0], i, 0);
                    let (gg, bb) = if gray { (rr, rr) } else { (get(&planes[1], i, 0), get(&planes[2], i, 0)) };
                    rgba.extend_from_slice(&[rr, gg, bb, get(&planes[3], i, 255)]);
                }
                l.rgba = rgba;
            }
            r.o = li_end;
            if let Some(first) = layers.first_mut()
                && !chans.first().is_some_and(|c| c.iter().any(|&(id, _)| id == -1))
                && first.section.is_none()
            {
                first.is_background = true;
            }
        }
        r.o = lm_end;
    }
    // Composite
    let mut composite = Vec::new();
    if let Some(comp) = r.u16() {
        let rowbytes = (w as usize * depth.max(1) as usize).div_ceil(8);
        let n = w as usize * h as usize;
        let mut planes: Vec<Vec<u8>> = Vec::new();
        let nch = ch.min(5) as usize;
        match comp {
            0 => {
                for _ in 0..nch {
                    let Some(d) = r.take(rowbytes * h as usize) else { break };
                    planes.push(to8(d, depth, n));
                }
            }
            1 => {
                let mut counts = Vec::new();
                for _ in 0..h as usize * nch {
                    counts.push(if psb { r.u32().unwrap_or(0) as usize } else { r.u16().unwrap_or(0) as usize });
                }
                for c in 0..nch {
                    let mut plane = Vec::with_capacity(rowbytes * h as usize);
                    for y in 0..h as usize {
                        let cnt = counts.get(c * h as usize + y).copied().unwrap_or(0);
                        let d = r.take(cnt).unwrap_or(&[]);
                        plane.extend(packbits(d, rowbytes));
                    }
                    planes.push(to8(&plane, depth, n));
                }
            }
            _ => {}
        }
        if !planes.is_empty() {
            composite.reserve(n * 4);
            for i in 0..n {
                let p = |k: usize| planes.get(k).and_then(|v| v.get(i)).copied();
                let px = match mode {
                    1 | 8 => {
                        let g = p(0).unwrap_or(0);
                        [g, g, g, p(1).unwrap_or(255)]
                    }
                    2 => {
                        let c = color_table.get(p(0).unwrap_or(0) as usize).copied().unwrap_or([0, 0, 0]);
                        [c[0], c[1], c[2], 255]
                    }
                    4 => {
                        // PSD speichert CMYK invertiert (0 = volle Farbe)
                        let (c, m, y, k) = (255 - p(0).unwrap_or(255), 255 - p(1).unwrap_or(255), 255 - p(2).unwrap_or(255), 255 - p(3).unwrap_or(255));
                        let ch = |v: u8| 255u16.saturating_sub((v as u16 + k as u16).min(255)) as u8;
                        [ch(c), ch(m), ch(y), p(4).unwrap_or(255)]
                    }
                    _ => [p(0).unwrap_or(0), p(1).unwrap_or(0), p(2).unwrap_or(0), p(3).unwrap_or(255)],
                };
                composite.extend_from_slice(&px);
            }
        }
    }
    Ok(PsdFile { psb, width: w, height: h, channels: ch, depth, mode, resources, layers, composite, color_table })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packbits_and_errors() {
        assert_eq!(packbits(&[2, 1, 2, 3, 254, 9], 6), vec![1, 2, 3, 9, 9, 9]);
        assert!(read(b"8BPS").is_err());
        assert!(read(b"nope").is_err());
    }
}
