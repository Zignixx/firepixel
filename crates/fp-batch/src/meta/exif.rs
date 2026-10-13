//! EXIF-Block (TIFF-Struktur ohne `Exif\0\0`) lesen, filtern und neu schreiben (SPEC-BATCH §3.7).
//!
//! Eigene Umsetzung statt `kamadak-exif` + `little_exif`: Gelesen werden IFD0, Exif-IFD, GPS-IFD,
//! Interop-IFD und IFD1 (Vorschaubild); geschrieben wird in der Byte-Reihenfolge der Quelle mit neu
//! berechneten Offsets. Werte bleiben als Rohbytes erhalten (kein Umkodieren). Die Ausrichtung wird
//! **an Ort und Stelle** gepatcht ([`set_orientation_in_place`]), damit ein unveränderter Block
//! Byte für Byte erhalten bleibt (Herstellerdaten mit absoluten Offsets gehen beim Neuschreiben
//! erfahrungsgemäß kaputt; deshalb wird nur neu geschrieben, wenn wirklich etwas entfernt wird).

use crate::item::ExifSummary;

/// Zeiger auf die Exif-IFD.
pub const TAG_EXIF_IFD: u16 = 0x8769;
/// Zeiger auf die GPS-IFD.
pub const TAG_GPS_IFD: u16 = 0x8825;
/// Zeiger auf die Interop-IFD (in der Exif-IFD).
pub const TAG_INTEROP_IFD: u16 = 0xA005;
/// Vorschaubild (IFD1): Offset und Länge.
pub const TAG_THUMB_OFFSET: u16 = 0x0201;
pub const TAG_THUMB_LENGTH: u16 = 0x0202;
pub const TAG_ORIENTATION: u16 = 0x0112;
pub const TAG_MAKE: u16 = 0x010F;
pub const TAG_MODEL: u16 = 0x0110;
pub const TAG_SOFTWARE: u16 = 0x0131;
pub const TAG_DATETIME: u16 = 0x0132;
pub const TAG_ARTIST: u16 = 0x013B;
pub const TAG_COPYRIGHT: u16 = 0x8298;
pub const TAG_DESCRIPTION: u16 = 0x010E;
pub const TAG_DATETIME_ORIGINAL: u16 = 0x9003;
pub const TAG_ISO: u16 = 0x8827;
pub const TAG_FNUMBER: u16 = 0x829D;
pub const TAG_LENS_MODEL: u16 = 0xA434;
pub const TAG_MAKER_NOTE: u16 = 0x927C;
pub const TAG_USER_COMMENT: u16 = 0x9286;
pub const TAG_XMP: u16 = 0x02BC;

/// Kamera-Angaben (Hersteller, Modell, Seriennummern, Objektiv, Herstellerdaten).
pub const CAMERA_TAGS: &[u16] = &[TAG_MAKE, TAG_MODEL, TAG_MAKER_NOTE, 0xA430, 0xA431, 0xA432, 0xA433, TAG_LENS_MODEL, 0xA435, 0xC62F];
/// Software-Angaben.
pub const SOFTWARE_TAGS: &[u16] = &[TAG_SOFTWARE, 0x013C, 0x000B];
/// Kommentare (Nutzerkommentar, Windows-Kommentar).
pub const COMMENT_TAGS: &[u16] = &[TAG_USER_COMMENT, 0x9C9C];

/// Ein Eintrag (Rohwert in der Byte-Reihenfolge der Quelle).
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub tag: u16,
    pub ty: u16,
    pub count: u32,
    pub data: Vec<u8>,
}

/// Eine IFD.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ifd {
    pub entries: Vec<Entry>,
}

impl Ifd {
    pub fn get(&self, tag: u16) -> Option<&Entry> {
        self.entries.iter().find(|e| e.tag == tag)
    }
    pub fn remove(&mut self, tags: &[u16]) -> bool {
        let n = self.entries.len();
        self.entries.retain(|e| !tags.contains(&e.tag));
        n != self.entries.len()
    }
    /// Eintrag setzen (ersetzt einen vorhandenen).
    pub fn set(&mut self, e: Entry) {
        self.entries.retain(|x| x.tag != e.tag);
        self.entries.push(e);
        self.entries.sort_by_key(|x| x.tag);
    }
}

/// Gelesene EXIF-Struktur.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Exif {
    pub little_endian: bool,
    pub ifd0: Ifd,
    pub exif: Option<Ifd>,
    pub gps: Option<Ifd>,
    pub interop: Option<Ifd>,
    pub ifd1: Option<Ifd>,
    pub thumbnail: Option<Vec<u8>>,
}

/// Größe eines TIFF-Typs in Bytes.
fn type_size(ty: u16) -> usize {
    match ty {
        1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 | 13 => 4,
        5 | 10 | 12 => 8,
        _ => 0,
    }
}

struct Reader<'a> {
    b: &'a [u8],
    le: bool,
}

impl Reader<'_> {
    fn u16(&self, i: usize) -> Option<u16> {
        let s: [u8; 2] = self.b.get(i..i + 2)?.try_into().ok()?;
        Some(if self.le { u16::from_le_bytes(s) } else { u16::from_be_bytes(s) })
    }
    fn u32(&self, i: usize) -> Option<u32> {
        let s: [u8; 4] = self.b.get(i..i + 4)?.try_into().ok()?;
        Some(if self.le { u32::from_le_bytes(s) } else { u32::from_be_bytes(s) })
    }
    /// Liest eine IFD; liefert sie und den Offset der nächsten.
    fn ifd(&self, off: usize, depth: u32) -> Option<(Ifd, u32)> {
        if depth > 4 || off < 8 {
            return None;
        }
        let n = self.u16(off)? as usize;
        if n > 1000 {
            return None;
        }
        let mut ifd = Ifd::default();
        for k in 0..n {
            let p = off + 2 + 12 * k;
            let tag = self.u16(p)?;
            let ty = self.u16(p + 2)?;
            let count = self.u32(p + 4)?;
            let size = type_size(ty);
            if size == 0 {
                continue;
            }
            let len = (count as usize).checked_mul(size)?;
            let data = if len <= 4 {
                self.b.get(p + 8..p + 8 + len)?.to_vec()
            } else {
                let o = self.u32(p + 8)? as usize;
                match self.b.get(o..o.checked_add(len)?) {
                    Some(d) => d.to_vec(),
                    None => continue,
                }
            };
            ifd.entries.push(Entry { tag, ty, count, data });
        }
        let next = self.u32(off + 2 + 12 * n).unwrap_or(0);
        Some((ifd, next))
    }
    fn value_u32(&self, e: &Entry) -> Option<u32> {
        match e.ty {
            3 => Reader { b: &e.data, le: self.le }.u16(0).map(u32::from),
            4 | 13 => Reader { b: &e.data, le: self.le }.u32(0),
            _ => None,
        }
    }
}

impl Exif {
    /// Liest einen EXIF-Block (mit oder ohne `Exif\0\0`).
    pub fn parse(data: &[u8]) -> Option<Exif> {
        let b = data.strip_prefix(b"Exif\0\0").unwrap_or(data);
        let le = match b.get(0..2)? {
            b"II" => true,
            b"MM" => false,
            _ => return None,
        };
        let r = Reader { b, le };
        if r.u16(2)? != 42 {
            return None;
        }
        let (ifd0, next) = r.ifd(r.u32(4)? as usize, 0)?;
        let sub = |tag: u16, ifd: &Ifd| ifd.get(tag).and_then(|e| r.value_u32(e)).and_then(|o| r.ifd(o as usize, 1)).map(|x| x.0);
        let exif = sub(TAG_EXIF_IFD, &ifd0);
        let gps = sub(TAG_GPS_IFD, &ifd0);
        let interop = exif.as_ref().and_then(|e| sub(TAG_INTEROP_IFD, e));
        let mut ifd1 = None;
        let mut thumbnail = None;
        if next != 0
            && let Some((i1, _)) = r.ifd(next as usize, 1)
        {
            let off = i1.get(TAG_THUMB_OFFSET).and_then(|e| r.value_u32(e));
            let len = i1.get(TAG_THUMB_LENGTH).and_then(|e| r.value_u32(e));
            if let (Some(o), Some(l)) = (off, len) {
                thumbnail = b.get(o as usize..(o as usize).saturating_add(l as usize)).map(<[u8]>::to_vec);
            }
            // Vorschaubild in Streifen (selten) wird nicht übernommen
            if thumbnail.is_some() {
                ifd1 = Some(i1);
            }
        }
        Some(Exif { little_endian: le, ifd0, exif, gps, interop, ifd1, thumbnail })
    }

    fn rd(&self) -> Reader<'static> {
        Reader { b: &[], le: self.little_endian }
    }

    /// Zahl (SHORT/LONG) eines Eintrags.
    pub fn number(&self, e: &Entry) -> Option<u32> {
        self.rd().value_u32(e)
    }

    /// Text (ASCII) eines Eintrags, ohne abschließende Nullen/Leerzeichen.
    pub fn text(e: &Entry) -> Option<String> {
        if e.ty != 2 && e.ty != 7 && e.ty != 1 {
            return None;
        }
        let s = String::from_utf8_lossy(&e.data).trim_end_matches(['\0', ' ']).trim().to_string();
        (!s.is_empty()).then_some(s)
    }

    /// Erster Wert eines RATIONAL-Eintrags.
    pub fn rational(&self, e: &Entry) -> Option<f64> {
        if e.ty != 5 && e.ty != 10 {
            return None;
        }
        let r = Reader { b: &e.data, le: self.little_endian };
        let (n, d) = (r.u32(0)?, r.u32(4)?);
        (d != 0).then(|| n as f64 / d as f64)
    }

    /// Eintrag in IFD0 oder Exif-IFD.
    pub fn find(&self, tag: u16) -> Option<&Entry> {
        self.ifd0.get(tag).or_else(|| self.exif.as_ref().and_then(|e| e.get(tag)))
    }

    /// Ausrichtung 1…8 (None = nicht gesetzt).
    pub fn orientation(&self) -> Option<u16> {
        self.ifd0.get(TAG_ORIENTATION).and_then(|e| self.number(e)).map(|v| v as u16).filter(|v| (1..=8).contains(v))
    }

    /// Zusammenfassung für Namensbausteine.
    pub fn summary(&self) -> ExifSummary {
        let text = |tag| self.find(tag).and_then(Exif::text);
        let date = text(TAG_DATETIME_ORIGINAL).or_else(|| text(TAG_DATETIME)).and_then(|s| chrono::NaiveDateTime::parse_from_str(&s, "%Y:%m:%d %H:%M:%S").ok());
        let aperture = self.find(TAG_FNUMBER).and_then(|e| self.rational(e)).filter(|f| *f > 0.0).map(|f| {
            let s = format!("{f:.1}");
            format!("f/{}", s.trim_end_matches(".0"))
        });
        ExifSummary {
            date_taken: date,
            make: text(TAG_MAKE),
            model: text(TAG_MODEL),
            lens: text(TAG_LENS_MODEL),
            iso: self.find(TAG_ISO).and_then(|e| self.number(e)),
            aperture,
            orientation: self.orientation(),
        }
    }

    /// Entfernt Einträge in allen IFDs. Liefert `true`, wenn sich etwas geändert hat.
    pub fn remove_tags(&mut self, tags: &[u16]) -> bool {
        let mut changed = self.ifd0.remove(tags);
        for i in [&mut self.exif, &mut self.interop, &mut self.ifd1, &mut self.gps].into_iter().flatten() {
            changed |= i.remove(tags);
        }
        changed
    }

    /// Hat der Block GPS-Daten?
    pub fn has_gps(&self) -> bool {
        self.gps.as_ref().is_some_and(|g| !g.entries.is_empty())
    }

    /// Text-Eintrag (ASCII) in IFD0 setzen bzw. bei leerem Text entfernen.
    pub fn set_ascii(&mut self, tag: u16, text: &str) {
        if text.is_empty() {
            self.ifd0.remove(&[tag]);
            return;
        }
        let mut data = text.as_bytes().to_vec();
        data.push(0);
        let count = data.len() as u32;
        self.ifd0.set(Entry { tag, ty: 2, count, data });
    }

    /// Ausrichtung setzen (IFD0).
    pub fn set_orientation(&mut self, v: u16) {
        let data = if self.little_endian { v.to_le_bytes() } else { v.to_be_bytes() }.to_vec();
        self.ifd0.set(Entry { tag: TAG_ORIENTATION, ty: 3, count: 1, data });
    }

    /// Leerer Block (Little Endian) für „Metadaten setzen“ ohne vorhandenes EXIF.
    pub fn new_empty() -> Exif {
        Exif { little_endian: true, ..Exif::default() }
    }

    /// Schreibt den Block neu (ohne `Exif\0\0`).
    pub fn write(&self) -> Vec<u8> {
        let le = self.little_endian;
        let mut w = Writer { buf: Vec::new(), le };
        w.buf.extend_from_slice(if le { b"II" } else { b"MM" });
        w.put16(42);
        w.put32(8);
        // IFD0 mit Zeigern auf die Unter-IFDs und IFD1
        let mut ifd0 = self.ifd0.clone();
        ifd0.remove(&[TAG_EXIF_IFD, TAG_GPS_IFD]);
        let mut exif = self.exif.clone();
        if let Some(e) = exif.as_mut() {
            e.remove(&[TAG_INTEROP_IFD]);
            if self.interop.as_ref().is_some_and(|i| !i.entries.is_empty()) {
                e.set(ptr_entry(TAG_INTEROP_IFD));
            }
        }
        let exif = exif.filter(|e| !e.entries.is_empty());
        let gps = self.gps.clone().filter(|g| !g.entries.is_empty());
        if exif.is_some() {
            ifd0.set(ptr_entry(TAG_EXIF_IFD));
        }
        if gps.is_some() {
            ifd0.set(ptr_entry(TAG_GPS_IFD));
        }
        let ifd1 = match (&self.ifd1, &self.thumbnail) {
            (Some(i), Some(_)) => Some(i.clone()),
            _ => None,
        };
        let p0 = w.ifd(&ifd0);
        if let Some(e) = &exif {
            let at = w.align();
            w.patch(p0.value_of(TAG_EXIF_IFD), at);
            let pe = w.ifd(e);
            if let Some(i) = self.interop.as_ref().filter(|i| !i.entries.is_empty()) {
                let at = w.align();
                w.patch(pe.value_of(TAG_INTEROP_IFD), at);
                w.ifd(i);
            }
        }
        if let Some(g) = &gps {
            let at = w.align();
            w.patch(p0.value_of(TAG_GPS_IFD), at);
            w.ifd(g);
        }
        if let (Some(i1), Some(th)) = (&ifd1, &self.thumbnail) {
            let at = w.align();
            w.patch(Some(p0.next), at);
            let mut i1 = i1.clone();
            i1.set(ptr_entry(TAG_THUMB_OFFSET));
            let len = th.len() as u32;
            i1.set(Entry { tag: TAG_THUMB_LENGTH, ty: 4, count: 1, data: if le { len.to_le_bytes() } else { len.to_be_bytes() }.to_vec() });
            let p1 = w.ifd(&i1);
            let at = w.align();
            w.patch(p1.value_of(TAG_THUMB_OFFSET), at);
            w.buf.extend_from_slice(th);
        }
        w.buf
    }
}

fn ptr_entry(tag: u16) -> Entry {
    Entry { tag, ty: 4, count: 1, data: vec![0; 4] }
}

/// Positionen in einer geschriebenen IFD.
struct Written {
    /// (Tag, Position des Wertfelds)
    values: Vec<(u16, usize)>,
    /// Position des „nächste IFD“-Felds.
    next: usize,
}

impl Written {
    fn value_of(&self, tag: u16) -> Option<usize> {
        self.values.iter().find(|v| v.0 == tag).map(|v| v.1)
    }
}

struct Writer {
    buf: Vec<u8>,
    le: bool,
}

impl Writer {
    fn put16(&mut self, v: u16) {
        self.buf.extend_from_slice(&if self.le { v.to_le_bytes() } else { v.to_be_bytes() });
    }
    fn put32(&mut self, v: u32) {
        self.buf.extend_from_slice(&if self.le { v.to_le_bytes() } else { v.to_be_bytes() });
    }
    fn align(&mut self) -> u32 {
        if self.buf.len() % 2 == 1 {
            self.buf.push(0);
        }
        self.buf.len() as u32
    }
    fn patch(&mut self, pos: Option<usize>, v: u32) {
        if let Some(p) = pos {
            let b = if self.le { v.to_le_bytes() } else { v.to_be_bytes() };
            self.buf[p..p + 4].copy_from_slice(&b);
        }
    }
    /// Schreibt eine IFD (nach Tag sortiert) mit ihren ausgelagerten Werten direkt dahinter.
    fn ifd(&mut self, ifd: &Ifd) -> Written {
        self.align();
        let mut entries: Vec<&Entry> = ifd.entries.iter().collect();
        entries.sort_by_key(|e| e.tag);
        let start = self.buf.len();
        let n = entries.len();
        let mut data_at = start + 2 + 12 * n + 4;
        let mut values = Vec::new();
        let mut blobs: Vec<(usize, &[u8])> = Vec::new();
        self.put16(n as u16);
        for e in &entries {
            self.put16(e.tag);
            self.put16(e.ty);
            self.put32(e.count);
            let pos = self.buf.len();
            values.push((e.tag, pos));
            if e.data.len() <= 4 {
                let mut v = e.data.clone();
                v.resize(4, 0);
                self.buf.extend_from_slice(&v);
            } else {
                data_at += data_at % 2;
                self.put32(data_at as u32);
                blobs.push((data_at, &e.data));
                data_at += e.data.len();
            }
        }
        let next = self.buf.len();
        self.put32(0);
        for (at, d) in blobs {
            self.buf.resize(at, 0);
            self.buf.extend_from_slice(d);
        }
        Written { values, next }
    }
}

/// Setzt die Ausrichtung im Rohblock auf `v`, ohne ihn neu zu schreiben (nur wenn das Tag als
/// SHORT in IFD0 steht). Liefert `true` bei Erfolg.
pub fn set_orientation_in_place(data: &mut [u8], v: u16) -> bool {
    let skip = if data.starts_with(b"Exif\0\0") { 6 } else { 0 };
    let b = &mut data[skip..];
    let le = match b.get(0..2) {
        Some(b"II") => true,
        Some(b"MM") => false,
        _ => return false,
    };
    let rd16 = |b: &[u8], i: usize| b.get(i..i + 2).map(|s| if le { u16::from_le_bytes([s[0], s[1]]) } else { u16::from_be_bytes([s[0], s[1]]) });
    let rd32 = |b: &[u8], i: usize| b.get(i..i + 4).map(|s| if le { u32::from_le_bytes([s[0], s[1], s[2], s[3]]) } else { u32::from_be_bytes([s[0], s[1], s[2], s[3]]) });
    let Some(off) = rd32(b, 4).map(|o| o as usize) else { return false };
    let Some(n) = rd16(b, off) else { return false };
    for k in 0..n as usize {
        let p = off + 2 + 12 * k;
        if rd16(b, p) == Some(TAG_ORIENTATION) && rd16(b, p + 2) == Some(3) {
            let bytes = if le { v.to_le_bytes() } else { v.to_be_bytes() };
            if let Some(s) = b.get_mut(p + 8..p + 10) {
                s.copy_from_slice(&bytes);
                return true;
            }
        }
    }
    false
}

/// Tag-Nummer aus Zahl oder Text (`"0x0112"`, `"274"`, bekannte Namen wie `"Make"`).
pub fn tag_from_value(v: &serde_json::Value) -> Option<u16> {
    if let Some(n) = v.as_u64() {
        return u16::try_from(n).ok();
    }
    let s = v.as_str()?.trim();
    if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return u16::from_str_radix(h, 16).ok();
    }
    if let Ok(n) = s.parse::<u16>() {
        return Some(n);
    }
    const NAMES: &[(&str, u16)] = &[
        ("make", TAG_MAKE),
        ("model", TAG_MODEL),
        ("software", TAG_SOFTWARE),
        ("datetime", TAG_DATETIME),
        ("artist", TAG_ARTIST),
        ("copyright", TAG_COPYRIGHT),
        ("imagedescription", TAG_DESCRIPTION),
        ("datetimeoriginal", TAG_DATETIME_ORIGINAL),
        ("usercomment", TAG_USER_COMMENT),
        ("makernote", TAG_MAKER_NOTE),
        ("lensmodel", TAG_LENS_MODEL),
        ("bodyserialnumber", 0xA431),
        ("cameraownername", 0xA430),
        ("orientation", TAG_ORIENTATION),
    ];
    let l = s.to_ascii_lowercase();
    NAMES.iter().find(|(n, _)| *n == l).map(|(_, t)| *t)
}
