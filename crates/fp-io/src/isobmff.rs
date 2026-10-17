//! ISO-BMFF-/HEIF-Container (ISO/IEC 14496-12, 23008-12, MIAF) für AVIF: Boxen, Items mit Ort
//! (`iloc`, auch `idat`), Eigenschaften (`ispe`, `colr`/nclx, `irot`, `imir`, `clap`, `auxC`, `av1C`,
//! `pixi`) in `ipma`-Reihenfolge, Referenzen (`dimg`, `auxl`, `prem`), Raster-Items (`grid`) und
//! Bildsequenz-Spuren (`moov`/`trak`, AVIF-Animation `avis`).
//!
//! Reines Lesen ohne Panik: jede Längenangabe wird geprüft, Fehler → [`ImportError::Corrupt`].

use std::collections::BTreeMap;

use crate::{ImportError, Result};

fn corrupt(s: &str) -> ImportError {
    ImportError::Corrupt(fp_i18n::tr!("err-import-heif-container", detail = s))
}

/// Byte-Leser mit Grenzprüfung (Big Endian).
struct Rd<'a> {
    d: &'a [u8],
    p: usize,
}

impl<'a> Rd<'a> {
    fn new(d: &'a [u8]) -> Self {
        Rd { d, p: 0 }
    }
    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .p
            .checked_add(n)
            .ok_or_else(|| corrupt(&fp_i18n::t("err-import-length")))?;
        let s = self
            .d
            .get(self.p..end)
            .ok_or_else(|| corrupt(&fp_i18n::t("err-import-d-box-short")))?;
        self.p = end;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.bytes(1)?[0])
    }
    fn u16(&mut self) -> Result<u16> {
        let b = self.bytes(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }
    fn u32(&mut self) -> Result<u32> {
        let b = self.bytes(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn u64(&mut self) -> Result<u64> {
        let b = self.bytes(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(u64::from_be_bytes(a))
    }
    /// Feld mit 0, 2, 4 oder 8 Bytes.
    fn un(&mut self, n: u8) -> Result<u64> {
        match n {
            0 => Ok(0),
            2 => Ok(self.u16()? as u64),
            4 => Ok(self.u32()? as u64),
            8 => self.u64(),
            _ => Err(corrupt(&fp_i18n::t("err-import-field-width"))),
        }
    }
    fn fourcc(&mut self) -> Result<[u8; 4]> {
        let b = self.bytes(4)?;
        Ok([b[0], b[1], b[2], b[3]])
    }
    /// Version und Flags einer FullBox.
    fn full(&mut self) -> Result<(u8, u32)> {
        let v = self.u32()?;
        Ok(((v >> 24) as u8, v & 0x00FF_FFFF))
    }
    fn rest(&mut self) -> &'a [u8] {
        let s = self.d.get(self.p..).unwrap_or(&[]);
        self.p = self.d.len();
        s
    }
    fn cstr(&mut self) -> Result<String> {
        let r = self.d.get(self.p..).unwrap_or(&[]);
        let n = r.iter().position(|&b| b == 0).unwrap_or(r.len());
        let s = String::from_utf8_lossy(&r[..n]).into_owned();
        self.p = (self.p + n + 1).min(self.d.len());
        Ok(s)
    }
}

/// Eine Box: Typ und Inhalt (ohne Kopf).
#[derive(Clone, Copy, Debug)]
pub struct BoxRef<'a> {
    pub kind: [u8; 4],
    pub body: &'a [u8],
}

/// Alle Boxen hintereinander in `d` (Rest < 8 Bytes wird ignoriert).
pub fn boxes(d: &[u8]) -> Result<Vec<BoxRef<'_>>> {
    let mut out = Vec::new();
    let mut p = 0usize;
    while p + 8 <= d.len() {
        let mut r = Rd { d, p };
        let size32 = r.u32()?;
        let kind = r.fourcc()?;
        let size = match size32 {
            0 => (d.len() - p) as u64,
            1 => r.u64()?,
            s => s as u64,
        };
        let mut hdr = r.p - p;
        if &kind == b"uuid" {
            hdr += 16;
        }
        if size < hdr as u64 || size > (d.len() - p) as u64 {
            return Err(corrupt(&fp_i18n::tr!(
                "err-import-box-length",
                name = String::from_utf8_lossy(&kind).to_string()
            )));
        }
        let size = size as usize;
        out.push(BoxRef {
            kind,
            body: &d[p + hdr..p + size],
        });
        p += size;
    }
    Ok(out)
}

fn find<'a>(list: &[BoxRef<'a>], kind: &[u8; 4]) -> Option<BoxRef<'a>> {
    list.iter().find(|b| &b.kind == kind).copied()
}

/// Farbbeschreibung `colr`/`nclx`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Nclx {
    pub primaries: u16,
    pub transfer: u16,
    pub matrix: u16,
    pub full_range: bool,
}

/// Saubere Blende `clap` (Zähler/Nenner).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CleanAperture {
    pub width: (u32, u32),
    pub height: (u32, u32),
    pub h_off: (i32, u32),
    pub v_off: (i32, u32),
}

/// Eine Item-Eigenschaft.
#[derive(Clone, Debug, PartialEq)]
pub enum Property {
    Ispe {
        width: u32,
        height: u32,
    },
    Colr(Nclx),
    /// Drehung gegen den Uhrzeigersinn in Vierteldrehungen.
    Irot(u8),
    /// Spiegelung: 0 = oben ↔ unten, 1 = links ↔ rechts (ISO/IEC 23008-12 6.5.12, wie libavif/heic-decoder).
    Imir(u8),
    Clap(CleanAperture),
    AuxC(String),
    /// `configOBUs` aus `av1C`.
    Av1C(Vec<u8>),
    Other([u8; 4]),
}

/// Ort eines Items (`iloc`).
#[derive(Clone, Debug, Default)]
struct Loc {
    method: u8,
    extents: Vec<(u64, u64)>,
}

/// Ein Item mit seinen Eigenschaften in `ipma`-Reihenfolge.
#[derive(Clone, Debug)]
pub struct Item {
    pub id: u32,
    pub kind: [u8; 4],
    pub props: Vec<Property>,
}

/// Geparster `meta`-Teil einer HEIF-Datei.
#[derive(Debug)]
pub struct Heif<'a> {
    file: &'a [u8],
    idat: &'a [u8],
    pub primary: u32,
    pub items: BTreeMap<u32, Item>,
    locs: BTreeMap<u32, Loc>,
    /// Referenzen (Art, von, nach).
    pub refs: Vec<([u8; 4], u32, Vec<u32>)>,
}

/// Rasterbeschreibung eines `grid`-Items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grid {
    pub rows: u32,
    pub columns: u32,
    pub width: u32,
    pub height: u32,
}

impl<'a> Heif<'a> {
    /// Liest den `meta`-Teil; `None`, wenn die Datei keinen hat (reine Sequenz).
    pub fn parse(file: &'a [u8]) -> Result<Option<Heif<'a>>> {
        let top = boxes(file)?;
        let Some(meta) = find(&top, b"meta") else {
            return Ok(None);
        };
        let mut r = Rd::new(meta.body);
        r.full()?;
        let kids = boxes(r.rest())?;
        let mut h = Heif {
            file,
            idat: &[],
            primary: 0,
            items: BTreeMap::new(),
            locs: BTreeMap::new(),
            refs: Vec::new(),
        };
        if let Some(b) = find(&kids, b"idat") {
            h.idat = b.body;
        }
        if let Some(b) = find(&kids, b"pitm") {
            let mut r = Rd::new(b.body);
            let (v, _) = r.full()?;
            h.primary = if v == 0 { r.u16()? as u32 } else { r.u32()? };
        }
        if let Some(b) = find(&kids, b"iinf") {
            let mut r = Rd::new(b.body);
            let (v, _) = r.full()?;
            let _n = if v == 0 { r.u16()? as u32 } else { r.u32()? };
            for e in boxes(r.rest())? {
                if &e.kind != b"infe" {
                    continue;
                }
                let mut r = Rd::new(e.body);
                let (v, _) = r.full()?;
                if v < 2 {
                    continue;
                }
                let id = if v == 2 { r.u16()? as u32 } else { r.u32()? };
                let _prot = r.u16()?;
                let kind = r.fourcc()?;
                h.items.insert(
                    id,
                    Item {
                        id,
                        kind,
                        props: Vec::new(),
                    },
                );
            }
        }
        if let Some(b) = find(&kids, b"iloc") {
            h.locs = parse_iloc(b.body)?;
        }
        if let Some(b) = find(&kids, b"iref") {
            let mut r = Rd::new(b.body);
            let (v, _) = r.full()?;
            for e in boxes(r.rest())? {
                let mut r = Rd::new(e.body);
                let from = if v == 0 { r.u16()? as u32 } else { r.u32()? };
                let n = r.u16()?;
                let mut to = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    to.push(if v == 0 { r.u16()? as u32 } else { r.u32()? });
                }
                h.refs.push((e.kind, from, to));
            }
        }
        if let Some(b) = find(&kids, b"iprp") {
            let kids = boxes(b.body)?;
            let props: Vec<Property> = match find(&kids, b"ipco") {
                Some(c) => boxes(c.body)?
                    .iter()
                    .map(parse_property)
                    .collect::<Result<_>>()?,
                None => Vec::new(),
            };
            for a in kids.iter().filter(|b| &b.kind == b"ipma") {
                let mut r = Rd::new(a.body);
                let (v, flags) = r.full()?;
                let n = r.u32()?;
                for _ in 0..n {
                    let id = if v < 1 { r.u16()? as u32 } else { r.u32()? };
                    let k = r.u8()?;
                    for _ in 0..k {
                        let idx = if flags & 1 != 0 {
                            (r.u16()? & 0x7FFF) as usize
                        } else {
                            (r.u8()? & 0x7F) as usize
                        };
                        if idx == 0 {
                            continue;
                        }
                        if let (Some(p), Some(it)) = (props.get(idx - 1), h.items.get_mut(&id)) {
                            it.props.push(p.clone());
                        }
                    }
                }
            }
        }
        Ok(Some(h))
    }

    /// Daten eines Items (alle Ausdehnungen aneinander).
    pub fn data(&self, id: u32) -> Result<Vec<u8>> {
        let loc = self
            .locs
            .get(&id)
            .ok_or_else(|| corrupt(&fp_i18n::t("err-import-d-item-no-location")))?;
        let src = match loc.method {
            0 => self.file,
            1 => self.idat,
            _ => {
                return Err(ImportError::Unsupported(fp_i18n::t(
                    "err-import-heif-construction",
                )));
            }
        };
        let mut out = Vec::new();
        for &(off, len) in &loc.extents {
            let (o, l) = (
                usize::try_from(off).map_err(|_| corrupt(&fp_i18n::t("err-import-offset")))?,
                usize::try_from(len).map_err(|_| corrupt(&fp_i18n::t("err-import-length")))?,
            );
            let end = if l == 0 && loc.extents.len() == 1 {
                src.len()
            } else {
                o.checked_add(l)
                    .ok_or_else(|| corrupt(&fp_i18n::t("err-import-length")))?
            };
            out.extend_from_slice(
                src.get(o..end)
                    .ok_or_else(|| corrupt(&fp_i18n::t("err-import-d-item-outside")))?,
            );
        }
        Ok(out)
    }

    /// Ziele der Referenz `kind` von `from` (Reihenfolge der Datei).
    pub fn refs_from(&self, from: u32, kind: &[u8; 4]) -> Vec<u32> {
        self.refs
            .iter()
            .filter(|r| &r.0 == kind && r.1 == from)
            .flat_map(|r| r.2.iter().copied())
            .collect()
    }

    /// Items, die mit `kind` auf `to` verweisen.
    pub fn refs_to(&self, to: u32, kind: &[u8; 4]) -> Vec<u32> {
        self.refs
            .iter()
            .filter(|r| &r.0 == kind && r.2.contains(&to))
            .map(|r| r.1)
            .collect()
    }

    /// Alpha-Hilfsbild des Items `id` (`auxl` + `auxC` mit Alpha-URN).
    pub fn alpha_of(&self, id: u32) -> Option<u32> {
        self.refs_to(id, b"auxl").into_iter().find(|a| {
            self.items.get(a).is_some_and(|it| {
                it.props.iter().any(|p| matches!(p, Property::AuxC(u) if u == "urn:mpeg:mpegB:cicp:systems:auxiliary:alpha" || u == "urn:mpeg:hevc:2015:auxid:1"))
            })
        })
    }

    /// Rasterbeschreibung eines `grid`-Items.
    pub fn grid(&self, id: u32) -> Result<Grid> {
        let d = self.data(id)?;
        let mut r = Rd::new(&d);
        let _v = r.u8()?;
        let flags = r.u8()?;
        let rows = r.u8()? as u32 + 1;
        let columns = r.u8()? as u32 + 1;
        let (width, height) = if flags & 1 != 0 {
            (r.u32()?, r.u32()?)
        } else {
            (r.u16()? as u32, r.u16()? as u32)
        };
        Ok(Grid {
            rows,
            columns,
            width,
            height,
        })
    }
}

fn parse_iloc(body: &[u8]) -> Result<BTreeMap<u32, Loc>> {
    let mut r = Rd::new(body);
    let (v, _) = r.full()?;
    let a = r.u8()?;
    let b = r.u8()?;
    let (off_size, len_size, base_size) = (a >> 4, a & 15, b >> 4);
    let index_size = if v == 1 || v == 2 { b & 15 } else { 0 };
    let n = if v < 2 { r.u16()? as u32 } else { r.u32()? };
    let mut out = BTreeMap::new();
    for _ in 0..n {
        let id = if v < 2 { r.u16()? as u32 } else { r.u32()? };
        let method = if v == 1 || v == 2 {
            (r.u16()? & 15) as u8
        } else {
            0
        };
        let _dref = r.u16()?;
        let base = r.un(base_size)?;
        let ne = r.u16()?;
        let mut extents = Vec::with_capacity(ne as usize);
        for _ in 0..ne {
            if index_size > 0 {
                r.un(index_size)?;
            }
            let off = r.un(off_size)?;
            let len = r.un(len_size)?;
            extents.push((
                base.checked_add(off)
                    .ok_or_else(|| corrupt(&fp_i18n::t("err-import-offset")))?,
                len,
            ));
        }
        out.insert(id, Loc { method, extents });
    }
    Ok(out)
}

fn parse_property(b: &BoxRef<'_>) -> Result<Property> {
    let mut r = Rd::new(b.body);
    Ok(match &b.kind {
        b"ispe" => {
            r.full()?;
            Property::Ispe {
                width: r.u32()?,
                height: r.u32()?,
            }
        }
        b"colr" => {
            let t = r.fourcc()?;
            if &t == b"nclx" {
                let primaries = r.u16()?;
                let transfer = r.u16()?;
                let matrix = r.u16()?;
                let full_range = r.u8()? & 0x80 != 0;
                Property::Colr(Nclx {
                    primaries,
                    transfer,
                    matrix,
                    full_range,
                })
            } else {
                Property::Other(*b"colr")
            }
        }
        b"irot" => Property::Irot(r.u8()? & 3),
        b"imir" => Property::Imir(r.u8()? & 1),
        b"clap" => {
            let width = (r.u32()?, r.u32()?);
            let height = (r.u32()?, r.u32()?);
            let h_off = (r.u32()? as i32, r.u32()?);
            let v_off = (r.u32()? as i32, r.u32()?);
            Property::Clap(CleanAperture {
                width,
                height,
                h_off,
                v_off,
            })
        }
        b"auxC" => {
            r.full()?;
            Property::AuxC(r.cstr()?)
        }
        b"av1C" => {
            r.bytes(4)?;
            Property::Av1C(r.rest().to_vec())
        }
        k => Property::Other(*k),
    })
}

/// Eine Spur einer Bildsequenz (`moov/trak`).
#[derive(Clone, Debug, Default)]
pub struct Track {
    pub id: u32,
    /// `hdlr`-Art (`pict`, `vide`, `auxv`).
    pub handler: [u8; 4],
    /// Art des ersten Sample-Eintrags (`av01`, `hvc1`).
    pub entry: [u8; 4],
    pub timescale: u32,
    /// Versatz und Länge je Sample (in der Datei).
    pub samples: Vec<(u64, u32)>,
    /// Dauer je Sample in Einheiten von `timescale`.
    pub durations: Vec<u32>,
    /// Spurreferenzen (`tref`): Art und Ziel-Spuren.
    pub refs: Vec<([u8; 4], Vec<u32>)>,
    /// `auxi`-Art des Sample-Eintrags bzw. der Hilfsspur (Alpha-URN).
    pub aux_type: Option<String>,
    /// Wiederholung aus der Bearbeitungsliste (`elst`-Flag 1 = endlos).
    pub repeat: bool,
}

/// Höchstzahl gelesener Samples je Spur.
pub const MAX_SAMPLES: usize = 10_000;

/// Alle Spuren der Datei (leer ohne `moov`).
pub fn tracks(file: &[u8]) -> Result<Vec<Track>> {
    let top = boxes(file)?;
    let Some(moov) = find(&top, b"moov") else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for trak in boxes(moov.body)?.iter().filter(|b| &b.kind == b"trak") {
        let tk = boxes(trak.body)?;
        let mut t = Track::default();
        if let Some(b) = find(&tk, b"tkhd") {
            let mut r = Rd::new(b.body);
            let (v, _) = r.full()?;
            r.bytes(if v == 1 { 16 } else { 8 })?;
            t.id = r.u32()?;
        }
        if let Some(b) = find(&tk, b"tref") {
            for e in boxes(b.body)? {
                let ids = e
                    .body
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|c| u32::from_be_bytes(*c))
                    .collect();
                t.refs.push((e.kind, ids));
            }
        }
        if let Some(b) = find(&tk, b"edts")
            && let Some(e) = find(&boxes(b.body)?, b"elst")
        {
            let mut r = Rd::new(e.body);
            let (_, flags) = r.full()?;
            t.repeat = flags & 1 != 0;
        }
        let Some(mdia) = find(&tk, b"mdia") else {
            continue;
        };
        let md = boxes(mdia.body)?;
        if let Some(b) = find(&md, b"mdhd") {
            let mut r = Rd::new(b.body);
            let (v, _) = r.full()?;
            r.bytes(if v == 1 { 16 } else { 8 })?;
            t.timescale = r.u32()?;
        }
        if let Some(b) = find(&md, b"hdlr") {
            let mut r = Rd::new(b.body);
            r.full()?;
            r.u32()?;
            t.handler = r.fourcc()?;
        }
        let Some(minf) = find(&md, b"minf") else {
            continue;
        };
        let Some(stbl) = find(&boxes(minf.body)?, b"stbl") else {
            continue;
        };
        let st = boxes(stbl.body)?;
        if let Some(b) = find(&st, b"stsd") {
            let mut r = Rd::new(b.body);
            r.full()?;
            r.u32()?;
            if let Some(e) = boxes(r.rest())?.first() {
                t.entry = e.kind;
                // VisualSampleEntry: 78 Bytes, danach Unterboxen (av1C, auxi …)
                if let Some(sub) = e.body.get(78..)
                    && let Ok(list) = boxes(sub)
                    && let Some(a) = find(&list, b"auxi")
                {
                    let mut r = Rd::new(a.body);
                    r.full()?;
                    t.aux_type = Some(r.cstr()?);
                }
            }
        }
        let sizes: Vec<u32> = match find(&st, b"stsz") {
            Some(b) => {
                let mut r = Rd::new(b.body);
                r.full()?;
                let fixed = r.u32()?;
                let n = (r.u32()? as usize).min(MAX_SAMPLES);
                if fixed != 0 {
                    vec![fixed; n]
                } else {
                    (0..n).map(|_| r.u32()).collect::<Result<_>>()?
                }
            }
            None => Vec::new(),
        };
        let offsets: Vec<u64> = if let Some(b) = find(&st, b"stco") {
            let mut r = Rd::new(b.body);
            r.full()?;
            let n = (r.u32()? as usize).min(MAX_SAMPLES);
            (0..n)
                .map(|_| r.u32().map(|v| v as u64))
                .collect::<Result<_>>()?
        } else if let Some(b) = find(&st, b"co64") {
            let mut r = Rd::new(b.body);
            r.full()?;
            let n = (r.u32()? as usize).min(MAX_SAMPLES);
            (0..n).map(|_| r.u64()).collect::<Result<_>>()?
        } else {
            Vec::new()
        };
        let stsc: Vec<(u32, u32)> = match find(&st, b"stsc") {
            Some(b) => {
                let mut r = Rd::new(b.body);
                r.full()?;
                let n = (r.u32()? as usize).min(MAX_SAMPLES);
                (0..n)
                    .map(|_| -> Result<(u32, u32)> {
                        let first = r.u32()?;
                        let per = r.u32()?;
                        r.u32()?;
                        Ok((first, per))
                    })
                    .collect::<Result<_>>()?
            }
            None => vec![(1, 1)],
        };
        // Samples den Chunks zuordnen
        let mut k = 0usize;
        'chunks: for (ci, &off) in offsets.iter().enumerate() {
            let chunk = ci as u32 + 1;
            let per = stsc
                .iter()
                .rev()
                .find(|e| e.0 <= chunk)
                .map(|e| e.1)
                .unwrap_or(1);
            let mut o = off;
            for _ in 0..per {
                let Some(&s) = sizes.get(k) else {
                    break 'chunks;
                };
                t.samples.push((o, s));
                o = o.saturating_add(s as u64);
                k += 1;
            }
        }
        if let Some(b) = find(&st, b"stts") {
            let mut r = Rd::new(b.body);
            r.full()?;
            let n = r.u32()?;
            for _ in 0..n {
                let count = r.u32()?;
                let delta = r.u32()?;
                for _ in 0..count {
                    if t.durations.len() >= t.samples.len() {
                        break;
                    }
                    t.durations.push(delta);
                }
            }
        }
        out.push(t);
    }
    Ok(out)
}

/// Daten eines Samples.
pub fn sample(file: &[u8], s: (u64, u32)) -> Result<&[u8]> {
    let o = usize::try_from(s.0).map_err(|_| corrupt(&fp_i18n::t("err-import-sample-offset")))?;
    file.get(o..o.saturating_add(s.1 as usize))
        .ok_or_else(|| corrupt(&fp_i18n::t("err-import-d-sample-outside")))
}

/// Eigenschafts-Transformationen (`clap`, `irot`, `imir` in Dateireihenfolge) auf ein RGBA-Bild.
pub fn apply_transforms(
    mut w: u32,
    mut h: u32,
    mut px: Vec<[u8; 4]>,
    props: &[Property],
) -> (u32, u32, Vec<[u8; 4]>) {
    for p in props {
        match *p {
            Property::Clap(c) => {
                let f = |n: (u32, u32)| {
                    if n.1 == 0 {
                        0.0
                    } else {
                        n.0 as f64 / n.1 as f64
                    }
                };
                let g = |n: (i32, u32)| {
                    if n.1 == 0 {
                        0.0
                    } else {
                        n.0 as f64 / n.1 as f64
                    }
                };
                let cw = f(c.width).round().clamp(1.0, w as f64) as u32;
                let ch = f(c.height).round().clamp(1.0, h as f64) as u32;
                let x0 = ((w as f64 - cw as f64) / 2.0 + g(c.h_off))
                    .round()
                    .clamp(0.0, (w - cw) as f64) as u32;
                let y0 = ((h as f64 - ch as f64) / 2.0 + g(c.v_off))
                    .round()
                    .clamp(0.0, (h - ch) as f64) as u32;
                let mut out = Vec::with_capacity(cw as usize * ch as usize);
                for y in y0..y0 + ch {
                    let row = (y * w) as usize;
                    out.extend_from_slice(&px[row + x0 as usize..row + (x0 + cw) as usize]);
                }
                (w, h, px) = (cw, ch, out);
            }
            Property::Irot(q) => {
                for _ in 0..q {
                    // 90° gegen den Uhrzeigersinn: neu (x, y) = alt (w − 1 − y, x)
                    let (nw, nh) = (h, w);
                    let mut out = vec![[0u8; 4]; px.len()];
                    for y in 0..nh {
                        for x in 0..nw {
                            out[(y * nw + x) as usize] = px[(x * w + (w - 1 - y)) as usize];
                        }
                    }
                    (w, h, px) = (nw, nh, out);
                }
            }
            Property::Imir(axis) => {
                if axis == 1 {
                    for row in px.chunks_exact_mut(w as usize) {
                        row.reverse();
                    }
                } else {
                    let rows: Vec<Vec<[u8; 4]>> = px
                        .chunks_exact(w as usize)
                        .rev()
                        .map(|r| r.to_vec())
                        .collect();
                    px = rows.concat();
                }
            }
            _ => {}
        }
    }
    (w, h, px)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transforms() {
        // 2×1: A B → irot 1 (90° ccw) ⇒ 1×2: B über A
        let a = [1, 0, 0, 255];
        let b = [2, 0, 0, 255];
        let (w, h, p) = apply_transforms(2, 1, vec![a, b], &[Property::Irot(1)]);
        assert_eq!((w, h), (1, 2));
        assert_eq!(p, vec![b, a]);
        let (_, _, p) = apply_transforms(2, 1, vec![a, b], &[Property::Imir(1)]);
        assert_eq!(p, vec![b, a]);
        let (w, h, p) = apply_transforms(2, 2, vec![a, b, b, a], &[Property::Imir(0)]);
        assert_eq!((w, h, p), (2, 2, vec![b, a, a, b]));
    }

    #[test]
    fn broken_boxes() {
        assert!(boxes(b"\0\0\0\x20ftyp").is_err());
        assert!(Heif::parse(b"\0\0\0\x08free").unwrap().is_none());
    }
}
