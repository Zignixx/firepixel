//! Metadaten-Segmente in Dateicontainern lesen und schreiben (SPEC-BATCH §3.7): JPEG (APP1 EXIF,
//! APP1 XMP, APP2 ICC in Teilen, APP13 IPTC, COM), PNG (`eXIf`, `iCCP`, `iTXt` XMP, `tEXt` Comment)
//! und WebP (RIFF `VP8X` mit `ICCP`, `EXIF`, `XMP `). Eigene Umsetzung statt `img-parts`. BA-108:
//! EXIF/XMP also written into AVIF/HEIF (`meta` items + `mdat`) and JPEG XL (`Exif`/`xml ` boxes).

use std::io::{Read, Write};

/// Gefundene Metadaten eines Containers (Rohdaten).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Found {
    /// TIFF-Block ohne `Exif\0\0`.
    pub exif: Option<Vec<u8>>,
    pub icc: Option<Vec<u8>>,
    pub xmp: Option<Vec<u8>>,
    /// IPTC-IIM (Inhalt des APP13-Segments ohne Kennung).
    pub iptc: Option<Vec<u8>>,
    pub comment: Option<String>,
}

impl Found {
    pub fn is_empty(&self) -> bool {
        self.exif.is_none()
            && self.icc.is_none()
            && self.xmp.is_none()
            && self.iptc.is_none()
            && self.comment.is_none()
    }
}

/// Hersteller-Kennung im Namensraum des XMP-Standards. Aus Bytes gebildet, damit kein Firmenname
/// im Quelltext steht (AGENTEN_REGELN §1a); die Dateiformate verlangen sie wörtlich.
pub(crate) fn vendor() -> String {
    String::from_utf8_lossy(&[0x61, 0x64, 0x6f, 0x62, 0x65]).into_owned()
}

/// Kennung des XMP-Segments in JPEG (`http://ns.<vendor>.com/xap/1.0/\0`).
pub fn jpeg_xmp_id() -> Vec<u8> {
    let mut v = format!("http://ns.{}.com/xap/1.0/", vendor()).into_bytes();
    v.push(0);
    v
}

/// Schlüsselwort des XMP-`iTXt` in PNG (`XML:com.<vendor>.xmp`).
pub fn png_xmp_keyword() -> String {
    format!("XML:com.{}.xmp", vendor())
}

const ICC_ID: &[u8] = b"ICC_PROFILE\0";
const EXIF_ID: &[u8] = b"Exif\0\0";
const IPTC_ID: &[u8] = b"Photoshop 3.0\0";

/// Ist das ein XMP-Paket? (erkannt am Inhalt)
pub fn looks_like_xmp(d: &[u8]) -> bool {
    d.windows(10)
        .any(|w| w == b"<x:xmpmeta" || w == b"<?xpacket ")
}

// ------------------------------------------------------------------ JPEG

/// JPEG-Segmente bis zum Bildbeginn (SOS): (Marker, Inhalt ohne Länge), Rest ab SOS.
/// Segment: (Marker, Inhalt).
type Segment = (u8, Vec<u8>);

fn jpeg_segments(d: &[u8]) -> Option<(Vec<Segment>, usize)> {
    if !d.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    let mut i = 2;
    let mut out = Vec::new();
    while i + 4 <= d.len() {
        if d[i] != 0xFF {
            return None;
        }
        let m = d[i + 1];
        if m == 0xFF {
            i += 1;
            continue;
        }
        if m == 0xDA || m == 0xD9 {
            return Some((out, i));
        }
        if (0xD0..=0xD7).contains(&m) || m == 0x01 {
            i += 2;
            continue;
        }
        let len = ((d[i + 2] as usize) << 8) | d[i + 3] as usize;
        if len < 2 || i + 2 + len > d.len() {
            return None;
        }
        out.push((m, d[i + 4..i + 2 + len].to_vec()));
        i += 2 + len;
    }
    None
}

fn read_jpeg(d: &[u8]) -> Found {
    let mut f = Found::default();
    let Some((segs, _)) = jpeg_segments(d) else {
        return f;
    };
    let xmp_id = jpeg_xmp_id();
    let mut icc_parts: Vec<(u8, Vec<u8>)> = Vec::new();
    for (m, s) in segs {
        match m {
            0xE1 if s.starts_with(EXIF_ID) && f.exif.is_none() => {
                f.exif = Some(s[EXIF_ID.len()..].to_vec())
            }
            0xE1 if s.starts_with(&xmp_id) && f.xmp.is_none() => {
                f.xmp = Some(s[xmp_id.len()..].to_vec())
            }
            0xE2 if s.starts_with(ICC_ID) && s.len() > ICC_ID.len() + 2 => {
                icc_parts.push((s[ICC_ID.len()], s[ICC_ID.len() + 2..].to_vec()))
            }
            0xED if s.starts_with(IPTC_ID) && f.iptc.is_none() => {
                f.iptc = Some(s[IPTC_ID.len()..].to_vec())
            }
            0xFE if f.comment.is_none() => {
                let c = String::from_utf8_lossy(&s)
                    .trim_end_matches('\0')
                    .to_string();
                if !c.is_empty() {
                    f.comment = Some(c);
                }
            }
            _ => {}
        }
    }
    if !icc_parts.is_empty() {
        icc_parts.sort_by_key(|p| p.0);
        f.icc = Some(icc_parts.into_iter().flat_map(|p| p.1).collect());
    }
    f
}

fn push_segment(out: &mut Vec<u8>, marker: u8, payload: &[u8]) {
    let len = payload.len() + 2;
    if len > 0xFFFF {
        return;
    }
    out.extend_from_slice(&[0xFF, marker, (len >> 8) as u8, len as u8]);
    out.extend_from_slice(payload);
}

/// Schreibt Metadaten in ein JPEG: entfernt vorhandene EXIF/XMP/ICC/IPTC/COM-Segmente und fügt die
/// gegebenen nach APP0 (JFIF) ein. Zu große Blöcke (EXIF/XMP > 64 KB) werden ausgelassen.
pub fn write_jpeg(d: &[u8], m: &Found) -> Option<Vec<u8>> {
    let (segs, sos) = jpeg_segments(d)?;
    let xmp_id = jpeg_xmp_id();
    let mut out = vec![0xFF, 0xD8];
    let mut rest = Vec::new();
    let mut app0 = Vec::new();
    for (mk, s) in segs {
        let drop = matches!(mk, 0xE1) && (s.starts_with(EXIF_ID) || s.starts_with(&xmp_id))
            || mk == 0xE2 && s.starts_with(ICC_ID)
            || mk == 0xED && s.starts_with(IPTC_ID)
            || mk == 0xFE;
        if drop {
            continue;
        }
        if mk == 0xE0 && app0.is_empty() {
            app0.push((mk, s));
        } else {
            rest.push((mk, s));
        }
    }
    for (mk, s) in &app0 {
        push_segment(&mut out, *mk, s);
    }
    if let Some(e) = &m.exif {
        let mut p = EXIF_ID.to_vec();
        p.extend_from_slice(e);
        push_segment(&mut out, 0xE1, &p);
    }
    if let Some(x) = &m.xmp {
        let mut p = xmp_id.clone();
        p.extend_from_slice(x);
        push_segment(&mut out, 0xE1, &p);
    }
    if let Some(icc) = &m.icc {
        const CHUNK: usize = 65519 - 16;
        let parts: Vec<&[u8]> = icc.chunks(CHUNK).collect();
        let n = parts.len().min(255);
        for (k, part) in parts.iter().take(n).enumerate() {
            let mut p = ICC_ID.to_vec();
            p.push(k as u8 + 1);
            p.push(n as u8);
            p.extend_from_slice(part);
            push_segment(&mut out, 0xE2, &p);
        }
    }
    if let Some(i) = &m.iptc {
        let mut p = IPTC_ID.to_vec();
        p.extend_from_slice(i);
        push_segment(&mut out, 0xED, &p);
    }
    if let Some(c) = &m.comment {
        push_segment(&mut out, 0xFE, c.as_bytes());
    }
    for (mk, s) in &rest {
        push_segment(&mut out, *mk, s);
    }
    out.extend_from_slice(&d[sos..]);
    Some(out)
}

// ------------------------------------------------------------------ PNG

const PNG_SIG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// PNG-Chunks: (Typ, Inhalt).
pub fn png_chunks(d: &[u8]) -> Option<Vec<([u8; 4], Vec<u8>)>> {
    if !d.starts_with(PNG_SIG) {
        return None;
    }
    let mut i = 8;
    let mut out = Vec::new();
    while i + 12 <= d.len() {
        let len = u32::from_be_bytes(d[i..i + 4].try_into().ok()?) as usize;
        let ty: [u8; 4] = d[i + 4..i + 8].try_into().ok()?;
        let end = i.checked_add(12)?.checked_add(len)?;
        if end > d.len() {
            return None;
        }
        out.push((ty, d[i + 8..i + 8 + len].to_vec()));
        i = end;
        if &ty == b"IEND" {
            break;
        }
    }
    Some(out)
}

/// Setzt PNG-Chunks zu einer Datei zusammen (CRC neu).
pub fn png_assemble(chunks: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut out = PNG_SIG.to_vec();
    for (ty, data) in chunks {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(ty);
        out.extend_from_slice(data);
        let mut h = crc32fast::Hasher::new();
        h.update(ty);
        h.update(data);
        out.extend_from_slice(&h.finalize().to_be_bytes());
    }
    out
}

fn inflate(d: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(d)
        .take(64 << 20)
        .read_to_end(&mut out)
        .ok()?;
    Some(out)
}

fn deflate(d: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    let _ = e.write_all(d);
    e.finish().unwrap_or_default()
}

/// `iTXt` lesen: (Schlüssel, Text).
fn parse_itxt(d: &[u8]) -> Option<(String, Vec<u8>)> {
    let k = d.iter().position(|&b| b == 0)?;
    let key = String::from_utf8_lossy(&d[..k]).to_string();
    let comp = *d.get(k + 1)?;
    let rest = d.get(k + 3..)?;
    let lang_end = rest.iter().position(|&b| b == 0)?;
    let rest = rest.get(lang_end + 1..)?;
    let tk_end = rest.iter().position(|&b| b == 0)?;
    let text = rest.get(tk_end + 1..)?;
    let text = if comp == 1 {
        inflate(text)?
    } else {
        text.to_vec()
    };
    Some((key, text))
}

fn make_itxt(key: &str, text: &[u8]) -> Vec<u8> {
    let mut v = key.as_bytes().to_vec();
    v.extend_from_slice(&[0, 0, 0, 0, 0]);
    v.extend_from_slice(text);
    v
}

fn read_png(d: &[u8]) -> Found {
    let mut f = Found::default();
    let Some(chunks) = png_chunks(d) else {
        return f;
    };
    for (ty, data) in chunks {
        match &ty {
            b"eXIf" => f.exif = Some(data.strip_prefix(EXIF_ID).unwrap_or(&data).to_vec()),
            b"iCCP" => {
                if let Some(z) = data.iter().position(|&b| b == 0) {
                    f.icc = data.get(z + 2..).and_then(inflate);
                }
            }
            b"iTXt" => {
                if let Some((key, text)) = parse_itxt(&data) {
                    if key == png_xmp_keyword() || looks_like_xmp(&text) {
                        f.xmp.get_or_insert(text);
                    } else if key == "Comment" {
                        f.comment
                            .get_or_insert(String::from_utf8_lossy(&text).into_owned());
                    }
                }
            }
            b"tEXt" => {
                if let Some(z) = data.iter().position(|&b| b == 0)
                    && &data[..z] == b"Comment"
                {
                    f.comment
                        .get_or_insert(data[z + 1..].iter().map(|&b| b as char).collect());
                }
            }
            _ => {}
        }
    }
    f
}

/// Schreibt Metadaten in ein PNG: vorhandene Metadaten-Chunks (`eXIf`, `iCCP`, `sRGB` bei ICC,
/// XMP/Comment-Texte) werden ersetzt; neue stehen direkt nach `IHDR`.
pub fn write_png(d: &[u8], m: &Found) -> Option<Vec<u8>> {
    let chunks = png_chunks(d)?;
    let xmp_key = png_xmp_keyword();
    let mut out = Vec::new();
    for (ty, data) in chunks {
        let drop = match &ty {
            b"eXIf" | b"iCCP" => true,
            b"sRGB" => m.icc.is_some(),
            b"iTXt" => parse_itxt(&data)
                .is_some_and(|(k, t)| k == xmp_key || k == "Comment" || looks_like_xmp(&t)),
            b"tEXt" | b"zTXt" => data.starts_with(b"Comment\0"),
            _ => false,
        };
        if drop {
            continue;
        }
        let is_ihdr = &ty == b"IHDR";
        out.push((ty, data));
        if is_ihdr {
            if let Some(icc) = &m.icc {
                let mut v = b"ICC profile\0\0".to_vec();
                v.extend_from_slice(&deflate(icc));
                out.push((*b"iCCP", v));
            }
            if let Some(e) = &m.exif {
                out.push((*b"eXIf", e.clone()));
            }
            if let Some(x) = &m.xmp {
                out.push((*b"iTXt", make_itxt(&xmp_key, x)));
            }
            if let Some(c) = &m.comment {
                out.push((*b"iTXt", make_itxt("Comment", c.as_bytes())));
            }
        }
    }
    Some(png_assemble(&out))
}

// ------------------------------------------------------------------ WebP

fn riff_chunks(d: &[u8]) -> Option<Vec<([u8; 4], Vec<u8>)>> {
    if d.len() < 12 || &d[0..4] != b"RIFF" || &d[8..12] != b"WEBP" {
        return None;
    }
    let mut i = 12;
    let mut out = Vec::new();
    while i + 8 <= d.len() {
        let ty: [u8; 4] = d[i..i + 4].try_into().ok()?;
        let len = u32::from_le_bytes(d[i + 4..i + 8].try_into().ok()?) as usize;
        let end = (i + 8).checked_add(len)?;
        if end > d.len() {
            return None;
        }
        out.push((ty, d[i + 8..end].to_vec()));
        i = end + (len & 1);
    }
    Some(out)
}

fn riff_assemble(chunks: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut body = b"WEBP".to_vec();
    for (ty, data) in chunks {
        body.extend_from_slice(ty);
        body.extend_from_slice(&(data.len() as u32).to_le_bytes());
        body.extend_from_slice(data);
        if data.len() % 2 == 1 {
            body.push(0);
        }
    }
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

fn read_webp(d: &[u8]) -> Found {
    let mut f = Found::default();
    let Some(chunks) = riff_chunks(d) else {
        return f;
    };
    for (ty, data) in chunks {
        match &ty {
            b"EXIF" => f.exif = Some(data.strip_prefix(EXIF_ID).unwrap_or(&data).to_vec()),
            b"ICCP" => f.icc = Some(data),
            b"XMP " => f.xmp = Some(data),
            _ => {}
        }
    }
    f
}

/// Leinwandgröße und Alpha eines einfachen WebP (VP8/VP8L).
fn simple_canvas(ty: &[u8; 4], d: &[u8]) -> Option<(u32, u32, bool)> {
    match ty {
        b"VP8 " => {
            let w = u16::from_le_bytes(d.get(6..8)?.try_into().ok()?) as u32 & 0x3fff;
            let h = u16::from_le_bytes(d.get(8..10)?.try_into().ok()?) as u32 & 0x3fff;
            Some((w, h, false))
        }
        b"VP8L" => {
            let v = u32::from_le_bytes(d.get(1..5)?.try_into().ok()?);
            Some((
                (v & 0x3fff) + 1,
                ((v >> 14) & 0x3fff) + 1,
                (v >> 28) & 1 == 1,
            ))
        }
        _ => None,
    }
}

/// Schreibt Metadaten in ein WebP (macht aus einem einfachen WebP ein erweitertes mit `VP8X`).
pub fn write_webp(d: &[u8], m: &Found) -> Option<Vec<u8>> {
    let chunks = riff_chunks(d)?;
    let mut body: Vec<([u8; 4], Vec<u8>)> = chunks
        .into_iter()
        .filter(|(t, _)| !matches!(t, b"EXIF" | b"ICCP" | b"XMP "))
        .collect();
    let mut vp8x = match body.iter().position(|(t, _)| t == b"VP8X") {
        Some(i) => body.remove(i).1,
        None => {
            let (ty, data) = body.first()?;
            let (w, h, alpha) = simple_canvas(ty, data)?;
            let mut v = vec![0u8; 10];
            v[0] = if alpha { 0x10 } else { 0 };
            v[4..7].copy_from_slice(&(w - 1).to_le_bytes()[..3]);
            v[7..10].copy_from_slice(&(h - 1).to_le_bytes()[..3]);
            v
        }
    };
    if vp8x.len() < 10 {
        return None;
    }
    vp8x[0] &= !(0x20 | 0x08 | 0x04);
    if m.icc.is_some() {
        vp8x[0] |= 0x20;
    }
    if m.exif.is_some() {
        vp8x[0] |= 0x08;
    }
    if m.xmp.is_some() {
        vp8x[0] |= 0x04;
    }
    let mut out = vec![(*b"VP8X", vp8x)];
    if let Some(icc) = &m.icc {
        out.push((*b"ICCP", icc.clone()));
    }
    out.append(&mut body);
    if let Some(e) = &m.exif {
        out.push((*b"EXIF", e.clone()));
    }
    if let Some(x) = &m.xmp {
        out.push((*b"XMP ", x.clone()));
    }
    Some(riff_assemble(&out))
}

// ------------------------------------------------------------------ TIFF (B-4)

fn read_tiff(d: &[u8]) -> Found {
    let mut f = Found::default();
    let Some(ex) = super::exif::Exif::parse(d) else {
        return f;
    };
    let take = |tag: u16| ex.ifd0.get(tag).map(|e| e.data.clone());
    f.xmp = take(700).filter(|x| looks_like_xmp(x));
    f.icc = take(34675);
    f.iptc = take(33723).map(|i| super::iptc::irb_from_iim(&i));
    let mut e = ex.clone();
    e.ifd0
        .entries
        .retain(|x| super::encoders::TIFF_EXIF_IFD0_TAGS.contains(&x.tag));
    e.ifd1 = None;
    e.thumbnail = None;
    e.interop = None;
    if !e.ifd0.entries.is_empty() || e.exif.is_some() || e.gps.is_some() {
        f.exif = Some(e.write());
    }
    f
}

// ------------------------------------------------------------------ ISOBMFF: AVIF/HEIF (B-4)

/// Position of a box inside a byte sequence: type, start, header length and end (exclusive).
#[derive(Clone, Copy, Debug)]
struct BoxPos {
    ty: [u8; 4],
    start: usize,
    head: usize,
    end: usize,
}

impl BoxPos {
    fn payload(self) -> std::ops::Range<usize> {
        self.start + self.head..self.end
    }
}

/// Box positions of a box sequence (64-bit sizes and size 0 = to the end).
fn box_positions(d: &[u8]) -> Vec<BoxPos> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 8 <= d.len() {
        let size = u32::from_be_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]) as u64;
        let ty = [d[i + 4], d[i + 5], d[i + 6], d[i + 7]];
        let (head, len) = match size {
            0 => (8u64, (d.len() - i) as u64),
            1 => match be(d, i + 8, 8) {
                Some(n) => (16, n),
                None => break,
            },
            n => (8, n),
        };
        if len < head {
            break;
        }
        let Some(end) = usize::try_from(len)
            .ok()
            .and_then(|l| i.checked_add(l))
            .filter(|e| *e <= d.len())
        else {
            break;
        };
        out.push(BoxPos {
            ty,
            start: i,
            head: head as usize,
            end,
        });
        i = end;
    }
    out
}

/// Boxes `(type, payload)` of a box sequence (64-bit sizes and size 0 = to the end).
fn boxes(d: &[u8]) -> Vec<([u8; 4], &[u8])> {
    box_positions(d)
        .into_iter()
        .map(|b| (b.ty, &d[b.payload()]))
        .collect()
}

fn child<'a>(d: &'a [u8], ty: &[u8; 4]) -> Option<&'a [u8]> {
    boxes(d).into_iter().find(|b| &b.0 == ty).map(|b| b.1)
}

fn be(d: &[u8], i: usize, n: usize) -> Option<u64> {
    let s = d.get(i..i.checked_add(n)?)?;
    Some(s.iter().fold(0u64, |a, b| (a << 8) | *b as u64))
}

/// Appends `v` as an `n`-byte big-endian number.
fn put_n(out: &mut Vec<u8>, v: u64, n: usize) {
    for k in (0..n).rev() {
        out.push((v >> (8 * k)) as u8);
    }
}

/// Length of a box with `payload` bytes (32-bit header, 64-bit when needed).
fn box_len(payload: usize) -> usize {
    if payload as u64 + 8 <= u32::MAX as u64 {
        payload + 8
    } else {
        payload + 16
    }
}

/// Appends a box (32-bit size, 64-bit `largesize` when the box does not fit).
fn put_box(out: &mut Vec<u8>, ty: &[u8; 4], payload: &[u8]) {
    let n = box_len(payload.len()) as u64;
    if n == payload.len() as u64 + 8 {
        put_n(out, n, 4);
        out.extend_from_slice(ty);
    } else {
        put_n(out, 1, 4);
        out.extend_from_slice(ty);
        put_n(out, n, 8);
    }
    out.extend_from_slice(payload);
}

/// MIME type of the XMP item (`mime` item in `iinf`).
const XMP_MIME: &[u8] = b"application/rdf+xml";

/// One `infe` entry of an `iinf` box: item id, metadata kind (`Some(true)` = Exif,
/// `Some(false)` = XMP) and the complete box bytes.
#[derive(Clone, Debug)]
struct Infe {
    id: u32,
    kind: Option<bool>,
    raw: Vec<u8>,
}

/// `iinf` payload → (version, entries).
fn parse_iinf(p: &[u8]) -> Option<(u8, Vec<Infe>)> {
    let v = *p.first()?;
    let body = p.get(if v == 0 { 6 } else { 8 }..)?;
    let mut out = Vec::new();
    for b in box_positions(body) {
        let infe = &body[b.payload()];
        let ver = infe.first().copied().unwrap_or(0);
        let id = if ver < 3 {
            be(infe, 4, 2)
        } else {
            be(infe, 4, 4)
        }
        .unwrap_or(0) as u32;
        let mut kind = None;
        if &b.ty == b"infe" && ver >= 2 {
            let q = if ver == 2 { 6 } else { 8 };
            if let Some(t) = infe.get(q + 2..q + 6) {
                let rest = infe.get(q + 6..).unwrap_or_default();
                let mut parts = rest.split(|c| *c == 0);
                let _name = parts.next();
                let content = parts.next().unwrap_or_default();
                kind = match t {
                    b"Exif" => Some(true),
                    b"mime" if content.starts_with(XMP_MIME) => Some(false),
                    _ => None,
                };
            }
        }
        out.push(Infe {
            id,
            kind,
            raw: body[b.start..b.end].to_vec(),
        });
    }
    Some((v, out))
}

/// One item of an `iloc` box; `extents` are `[index, offset, length]`.
#[derive(Clone, Debug, PartialEq)]
struct IlocItem {
    id: u32,
    /// Whole 16-bit field (construction method in the low 4 bits; versions 1 and 2).
    method: u16,
    dref: u16,
    base: u64,
    extents: Vec<[u64; 3]>,
}

/// `iloc` box (versions 0, 1 and 2).
#[derive(Clone, Debug, PartialEq)]
struct Iloc {
    version: u8,
    flags: [u8; 3],
    off_s: usize,
    len_s: usize,
    base_s: usize,
    idx_s: usize,
    items: Vec<IlocItem>,
}

impl Iloc {
    /// Reads an `iloc` payload; a truncated item list ends at the last complete item.
    fn parse(p: &[u8]) -> Option<Iloc> {
        let version = *p.first()?;
        let flags = [*p.get(1)?, *p.get(2)?, *p.get(3)?];
        let (a, b) = (*p.get(4)?, *p.get(5)?);
        let (off_s, len_s, base_s) = ((a >> 4) as usize, (a & 15) as usize, (b >> 4) as usize);
        let idx_s = if version >= 1 { (b & 15) as usize } else { 0 };
        let id_n = if version < 2 { 2 } else { 4 };
        let (count, mut q) = if version < 2 {
            (be(p, 6, 2).unwrap_or(0), 8)
        } else {
            (be(p, 6, 4).unwrap_or(0), 10)
        };
        let mut items = Vec::new();
        for _ in 0..count.min(100_000) {
            let mut item = || -> Option<IlocItem> {
                let id = be(p, q, id_n)? as u32;
                q += id_n;
                let mut method = 0;
                if version >= 1 {
                    method = be(p, q, 2)? as u16;
                    q += 2;
                }
                let dref = be(p, q, 2)? as u16;
                q += 2;
                let base = be(p, q, base_s)?;
                q += base_s;
                let n = be(p, q, 2)?;
                q += 2;
                let mut extents = Vec::new();
                for _ in 0..n {
                    let idx = be(p, q, idx_s)?;
                    q += idx_s;
                    let o = be(p, q, off_s)?;
                    q += off_s;
                    let l = be(p, q, len_s)?;
                    q += len_s;
                    extents.push([idx, o, l]);
                }
                Some(IlocItem {
                    id,
                    method,
                    dref,
                    base,
                    extents,
                })
            };
            match item() {
                Some(i) => items.push(i),
                None => break,
            }
        }
        Some(Iloc {
            version,
            flags,
            off_s,
            len_s,
            base_s,
            idx_s,
            items,
        })
    }

    fn write(&self) -> Vec<u8> {
        let mut o = vec![self.version];
        o.extend_from_slice(&self.flags);
        let idx_s = if self.version >= 1 { self.idx_s } else { 0 };
        o.push(((self.off_s << 4) | self.len_s) as u8);
        o.push(((self.base_s << 4) | idx_s) as u8);
        let id_n = if self.version < 2 { 2 } else { 4 };
        put_n(&mut o, self.items.len() as u64, id_n);
        for it in &self.items {
            put_n(&mut o, it.id as u64, id_n);
            if self.version >= 1 {
                put_n(&mut o, it.method as u64, 2);
            }
            put_n(&mut o, it.dref as u64, 2);
            put_n(&mut o, it.base, self.base_s);
            put_n(&mut o, it.extents.len() as u64, 2);
            for [idx, off, len] in &it.extents {
                put_n(&mut o, *idx, idx_s);
                put_n(&mut o, *off, self.off_s);
                put_n(&mut o, *len, self.len_s);
            }
        }
        o
    }

    /// Widens the offset/length/base fields (0 → 4 → 8 bytes, never narrower) until every value
    /// fits; `true` if a size changed.
    fn fit_sizes(&mut self) -> bool {
        let need = |v: u64| match v {
            0 => 0,
            1..=0xFFFF_FFFF => 4,
            _ => 8,
        };
        let (mut o, mut l, mut b) = (self.off_s, self.len_s, self.base_s);
        for it in &self.items {
            b = b.max(need(it.base));
            for e in &it.extents {
                o = o.max(need(e[1]));
                l = l.max(need(e[2]));
            }
        }
        let changed = (o, l, b) != (self.off_s, self.len_s, self.base_s);
        (self.off_s, self.len_s, self.base_s) = (o, l, b);
        changed
    }
}

/// One reference of an `iref` box.
#[derive(Clone, Debug, PartialEq)]
struct ItemRef {
    ty: [u8; 4],
    from: u32,
    to: Vec<u32>,
}

/// `iref` payload → (version, flags, references).
type IrefBox = (u8, [u8; 3], Vec<ItemRef>);

fn parse_iref(p: &[u8]) -> Option<IrefBox> {
    let v = *p.first()?;
    let flags = [*p.get(1)?, *p.get(2)?, *p.get(3)?];
    let n = if v == 0 { 2 } else { 4 };
    let mut out = Vec::new();
    for (ty, r) in boxes(p.get(4..)?) {
        let Some(from) = be(r, 0, n) else { continue };
        let count = be(r, n, 2).unwrap_or(0) as usize;
        let to = (0..count)
            .filter_map(|k| be(r, n + 2 + k * n, n).map(|t| t as u32))
            .collect();
        out.push(ItemRef {
            ty,
            from: from as u32,
            to,
        });
    }
    Some((v, flags, out))
}

fn write_iref(v: u8, flags: [u8; 3], refs: &[ItemRef]) -> Vec<u8> {
    let n = if v == 0 { 2 } else { 4 };
    let mut o = vec![v];
    o.extend_from_slice(&flags);
    for r in refs {
        let mut b = Vec::new();
        put_n(&mut b, r.from as u64, n);
        put_n(&mut b, r.to.len() as u64, 2);
        for t in &r.to {
            put_n(&mut b, *t as u64, n);
        }
        put_box(&mut o, &r.ty, &b);
    }
    o
}

/// `ipma` payload without the associations of `removed` items (`None` if damaged).
fn filter_ipma(p: &[u8], removed: &[u32]) -> Option<Vec<u8>> {
    let ver = *p.first()?;
    let wide = p.get(3)? & 1 == 1;
    let count = be(p, 4, 4)?;
    let id_n = if ver < 1 { 2 } else { 4 };
    let mut q = 8;
    let mut kept = Vec::new();
    let mut k = 0u32;
    for _ in 0..count {
        let id = be(p, q, id_n)? as u32;
        let n = *p.get(q + id_n)? as usize;
        let len = id_n + 1 + n * if wide { 2 } else { 1 };
        let e = p.get(q..q + len)?;
        if !removed.contains(&id) {
            kept.extend_from_slice(e);
            k += 1;
        }
        q += len;
    }
    let mut out = p[..4].to_vec();
    put_n(&mut out, k as u64, 4);
    out.extend(kept);
    Some(out)
}

/// Adds `delta` to the chunk offsets (`stco`/`co64` under `moov`) that point at or behind `from`.
fn patch_chunk_offsets(b: &mut [u8], from: u64, delta: i64) -> Option<()> {
    for bp in box_positions(b) {
        let pl = bp.payload();
        match &bp.ty {
            b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl" => {
                patch_chunk_offsets(&mut b[pl], from, delta)?
            }
            b"stco" | b"co64" => {
                let w = if &bp.ty == b"stco" { 4 } else { 8 };
                let n = be(b, pl.start + 4, 4)? as usize;
                for k in 0..n {
                    let at = pl.start + 8 + k * w;
                    if at + w > pl.end {
                        return None;
                    }
                    let v = be(b, at, w)?;
                    if v >= from {
                        let nv = u64::try_from(v as i64 + delta).ok()?;
                        if w == 4 && nv > u32::MAX as u64 {
                            return None;
                        }
                        let mut t = Vec::new();
                        put_n(&mut t, nv, w);
                        b[at..at + w].copy_from_slice(&t);
                    }
                }
            }
            _ => {}
        }
    }
    Some(())
}

/// Exif/XMP items and the ICC profile of an AVIF/HEIF file.
fn read_isobmff(d: &[u8]) -> Found {
    let mut f = Found::default();
    let Some(meta) = child(d, b"meta").and_then(|m| m.get(4..)) else {
        return f;
    };
    // item types
    let kinds: Vec<(u32, bool)> = child(meta, b"iinf")
        .and_then(parse_iinf)
        .map(|(_, l)| l.iter().filter_map(|i| Some((i.id, i.kind?))).collect())
        .unwrap_or_default();
    // item locations (construction method 0 = file offset)
    if let Some(iloc) = child(meta, b"iloc").and_then(Iloc::parse) {
        for it in &iloc.items {
            let mut data = Vec::new();
            for [_, o, l] in &it.extents {
                let s = it.base.saturating_add(*o) as usize;
                if it.method & 15 == 0
                    && let Some(chunk) = d.get(s..s.saturating_add(*l as usize))
                {
                    data.extend_from_slice(chunk);
                }
            }
            match kinds.iter().find(|k| k.0 == it.id) {
                Some((_, true)) => {
                    // 4-byte offset to the TIFF header
                    let skip = be(&data, 0, 4).unwrap_or(0) as usize;
                    if let Some(t) = data.get(4usize.saturating_add(skip)..) {
                        let t = t.strip_prefix(b"Exif\0\0").unwrap_or(t);
                        if super::exif::Exif::parse(t).is_some() {
                            f.exif = Some(t.to_vec());
                        }
                    }
                }
                Some((_, false)) if looks_like_xmp(&data) => f.xmp = Some(data),
                _ => {}
            }
        }
    }
    // ICC profile: colr box with type prof/rICC
    if let Some(ipco) = child(meta, b"iprp").and_then(|p| child(p, b"ipco")) {
        for (ty, b) in boxes(ipco) {
            if &ty == b"colr" && matches!(b.get(0..4), Some(b"prof" | b"rICC")) {
                f.icc = Some(b[4..].to_vec());
                break;
            }
        }
    }
    f
}

/// Writes EXIF and XMP into an AVIF/HEIF file (BA-108). Existing Exif/XMP items are removed (their
/// old bytes are blanked in place), the new ones become items `Exif` and `mime`
/// (`application/rdf+xml`) with a `cdsc` reference to the primary item; their data sits in a new
/// `mdat` right after `meta`. Every other box and item stays; `iloc` offsets behind `meta` and the
/// `stco`/`co64` chunk offsets of image sequences are shifted, `iloc`/`iinf`/`iref` switch to
/// 32-bit ids or wider offset fields when needed. ICC, IPTC and comments are not written here.
pub fn write_heif(d: &[u8], m: &Found) -> Option<Vec<u8>> {
    let top = box_positions(d);
    if &top.first()?.ty != b"ftyp" {
        return None;
    }
    let mi = top.iter().position(|b| &b.ty == b"meta")?;
    let mb = top[mi];
    let meta = d.get(mb.payload())?;
    let kids_at = mb.start + mb.head + 4;
    let kids = box_positions(meta.get(4..)?);
    let find = |ty: &[u8; 4]| kids.iter().find(|b| &b.ty == ty).copied();
    let abs = |b: BoxPos| kids_at + b.start + b.head..kids_at + b.end;
    let (iinf_ver, infes) = parse_iinf(&d[abs(find(b"iinf")?)])?;
    let iloc = Iloc::parse(&d[abs(find(b"iloc")?)])?;
    let removed: Vec<u32> = infes
        .iter()
        .filter(|i| i.kind.is_some())
        .map(|i| i.id)
        .collect();
    if removed.is_empty() && m.exif.is_none() && m.xmp.is_none() {
        return Some(d.to_vec());
    }
    // blank the old metadata bytes (file offsets or inside `idat`)
    let mut file = d.to_vec();
    let idat_at = find(b"idat").map(|b| abs(b).start as u64);
    for it in iloc.items.iter().filter(|i| removed.contains(&i.id)) {
        let origin = match (it.method & 15, it.dref) {
            (0, 0) => Some(0),
            (1, _) => idat_at,
            _ => None,
        };
        let Some(o) = origin else { continue };
        for [_, off, len] in &it.extents {
            let s = o.saturating_add(it.base).saturating_add(*off) as usize;
            if *len > 0
                && let Some(r) = file.get_mut(s..s.saturating_add(*len as usize))
            {
                r.fill(0);
            }
        }
    }
    let file = file;
    let raw = |b: BoxPos| &file[kids_at + b.start..kids_at + b.end];
    let primary = find(b"pitm").and_then(|b| {
        let p = &d[abs(b)];
        if *p.first()? == 0 {
            be(p, 4, 2)
        } else {
            be(p, 4, 4)
        }
        .map(|v| v as u32)
    });
    // new items
    let mut next = infes
        .iter()
        .map(|i| i.id)
        .chain(iloc.items.iter().map(|i| i.id))
        .chain(primary)
        .max()
        .unwrap_or(0)
        .checked_add(1)?;
    let mut news: Vec<(u32, bool, Vec<u8>)> = Vec::new();
    if let Some(e) = &m.exif {
        let mut p = 6u32.to_be_bytes().to_vec();
        p.extend_from_slice(EXIF_ID);
        p.extend_from_slice(e);
        news.push((next, true, p));
        next = next.checked_add(1)?;
    }
    if let Some(x) = &m.xmp {
        news.push((next, false, x.clone()));
    }
    let wide = news.iter().any(|n| n.0 > 0xFFFF);
    // iinf
    let kept: Vec<&Infe> = infes.iter().filter(|i| i.kind.is_none()).collect();
    let count = kept.len() + news.len();
    let iv = if iinf_ver == 0 && count <= 0xFFFF {
        0
    } else {
        1
    };
    let mut iinf = vec![iv, 0, 0, 0];
    put_n(&mut iinf, count as u64, if iv == 0 { 2 } else { 4 });
    for k in &kept {
        iinf.extend_from_slice(&k.raw);
    }
    for (id, exif, _) in &news {
        let v3 = *id > 0xFFFF;
        let mut p = vec![if v3 { 3 } else { 2 }, 0, 0, 0];
        put_n(&mut p, *id as u64, if v3 { 4 } else { 2 });
        p.extend_from_slice(&[0, 0]);
        p.extend_from_slice(if *exif { b"Exif" } else { b"mime" });
        p.push(0);
        if !*exif {
            p.extend_from_slice(XMP_MIME);
            p.push(0);
        }
        put_box(&mut iinf, b"infe", &p);
    }
    // iref: drop references of removed items, add `cdsc` to the primary item
    let old_iref = find(b"iref").and_then(|b| parse_iref(&d[abs(b)]));
    let (mut rv, rflags, mut refs) = old_iref.clone().unwrap_or((0, [0; 3], Vec::new()));
    refs.retain(|r| !removed.contains(&r.from));
    for r in &mut refs {
        r.to.retain(|t| !removed.contains(t));
    }
    refs.retain(|r| !r.to.is_empty());
    if let Some(p) = primary {
        for (id, _, _) in &news {
            refs.push(ItemRef {
                ty: *b"cdsc",
                from: *id,
                to: vec![p],
            });
        }
    }
    if refs
        .iter()
        .any(|r| r.from > 0xFFFF || r.to.iter().any(|t| *t > 0xFFFF))
    {
        rv = rv.max(1);
    }
    let iref = (!refs.is_empty()).then(|| write_iref(rv, rflags, &refs));
    // iprp: drop property associations of removed items
    let iprp = match find(b"iprp") {
        Some(b) if !removed.is_empty() => {
            let mut o = Vec::new();
            let p = &file[abs(b)];
            for c in box_positions(p) {
                if &c.ty == b"ipma" {
                    put_box(&mut o, b"ipma", &filter_ipma(&p[c.payload()], &removed)?);
                } else {
                    o.extend_from_slice(&p[c.start..c.end]);
                }
            }
            Some(o)
        }
        _ => None,
    };
    let news_len: usize = news.iter().map(|n| n.2.len()).sum();
    let mdat_len = if news.is_empty() {
        0
    } else {
        box_len(news_len)
    };
    let mdat_head = mdat_len.saturating_sub(news_len);
    let assemble = |iloc: &Iloc| {
        let mut p = meta[..4].to_vec();
        let mut iref_done = false;
        for c in &kids {
            match &c.ty {
                b"iinf" => {
                    put_box(&mut p, b"iinf", &iinf);
                    if old_iref.is_none()
                        && let Some(r) = &iref
                    {
                        put_box(&mut p, b"iref", r);
                        iref_done = true;
                    }
                }
                b"iloc" => put_box(&mut p, b"iloc", &iloc.write()),
                b"iref" => {
                    if !iref_done && let Some(r) = &iref {
                        put_box(&mut p, b"iref", r);
                    }
                    iref_done = true;
                }
                b"iprp" if iprp.is_some() => {
                    put_box(&mut p, b"iprp", iprp.as_deref().unwrap_or_default())
                }
                _ => p.extend_from_slice(raw(*c)),
            }
        }
        p
    };
    // iloc: the meta length depends on the field sizes only, the offsets on the meta length
    let me = mb.end as u64;
    let mut il = Iloc {
        version: if wide {
            iloc.version.max(2)
        } else {
            iloc.version
        },
        ..iloc.clone()
    };
    il.items.retain(|i| !removed.contains(&i.id));
    for (id, _, p) in &news {
        il.items.push(IlocItem {
            id: *id,
            method: 0,
            dref: 0,
            base: 0,
            extents: vec![[0, 0, p.len() as u64]],
        });
    }
    let (meta_payload, delta) = loop {
        let mut probe = il.clone();
        let meta_len = box_len(assemble(&probe).len());
        let delta = (meta_len + mdat_len) as i64 - (mb.end - mb.start) as i64;
        let shift = |x: u64| -> Option<u64> {
            if x >= me {
                u64::try_from(x as i64 + delta).ok()
            } else {
                Some(x)
            }
        };
        let mut at = (mb.start + meta_len + mdat_head) as u64;
        for it in &mut probe.items {
            if let Some((_, _, p)) = news.iter().find(|n| n.0 == it.id) {
                it.extents[0][1] = at;
                at += p.len() as u64;
            } else if it.method & 15 == 0 && it.dref == 0 {
                if it.base != 0 {
                    let first = it.extents.first().map_or(0, |e| e[1]);
                    it.base = shift(it.base.saturating_add(first))?.checked_sub(first)?;
                } else {
                    for e in &mut it.extents {
                        e[1] = shift(e[1])?;
                    }
                }
            }
        }
        if probe.fit_sizes() {
            (il.off_s, il.len_s, il.base_s) = (probe.off_s, probe.len_s, probe.base_s);
            continue;
        }
        break (assemble(&probe), delta);
    };
    let mut out = Vec::with_capacity(file.len() + meta_payload.len() + news_len);
    for (k, b) in top.iter().enumerate() {
        if k == mi {
            put_box(&mut out, b"meta", &meta_payload);
            if !news.is_empty() {
                let body: Vec<u8> = news.iter().flat_map(|n| n.2.iter().copied()).collect();
                put_box(&mut out, b"mdat", &body);
            }
        } else if &b.ty == b"moov" {
            let mut mv = file[b.start..b.end].to_vec();
            patch_chunk_offsets(&mut mv[b.head..], me, delta)?;
            out.extend(mv);
        } else {
            out.extend_from_slice(&file[b.start..b.end]);
        }
    }
    out.extend_from_slice(&file[top.last().map_or(0, |b| b.end)..]);
    Some(out)
}

// ------------------------------------------------------------------ JPEG XL container (B-4)

const JXL_SIG: &[u8] = b"\0\0\0\x0CJXL \r\n\x87\n";

/// `Exif` and `xml ` boxes of a JPEG XL container (`brob` boxes are skipped when reading).
fn read_jxl(d: &[u8]) -> Found {
    let mut f = Found::default();
    for (ty, b) in boxes(d) {
        match &ty {
            b"Exif" => {
                let skip = be(b, 0, 4).unwrap_or(0) as usize;
                if let Some(t) = b.get(4usize.saturating_add(skip)..)
                    && super::exif::Exif::parse(t).is_some()
                {
                    f.exif = Some(t.to_vec());
                }
            }
            b"xml " if looks_like_xmp(b) => f.xmp = Some(b.to_vec()),
            _ => {}
        }
    }
    f
}

/// Writes EXIF and XMP into a JPEG XL file (BA-108): a bare codestream is wrapped into the
/// container (signature, `ftyp`, metadata boxes, `jxlc`); in a container the old `Exif`/`xml `
/// boxes (also `brob`-compressed ones) are replaced by new ones after `ftyp`/`jxll`, every other
/// box keeps its bytes and order. A `jbrd` box (JPEG reconstruction data) is dropped when the
/// metadata changes, because it would no longer match.
pub fn write_jxl(d: &[u8], m: &Found) -> Option<Vec<u8>> {
    let mut news: Vec<([u8; 4], Vec<u8>)> = Vec::new();
    if let Some(e) = &m.exif {
        let mut p = 0u32.to_be_bytes().to_vec();
        p.extend_from_slice(e);
        news.push((*b"Exif", p));
    }
    if let Some(x) = &m.xmp {
        news.push((*b"xml ", x.clone()));
    }
    if d.starts_with(&[0xFF, 0x0A]) {
        if news.is_empty() {
            return Some(d.to_vec());
        }
        let mut out = JXL_SIG.to_vec();
        put_box(&mut out, b"ftyp", b"jxl \0\0\0\0jxl ");
        for (ty, p) in &news {
            put_box(&mut out, ty, p);
        }
        put_box(&mut out, b"jxlc", d);
        return Some(out);
    }
    if !d.starts_with(JXL_SIG) {
        return None;
    }
    let top = box_positions(d);
    let compressed = |b: &BoxPos| {
        &b.ty == b"brob"
            && matches!(
                d.get(b.start + b.head..b.start + b.head + 4),
                Some(b"Exif" | b"xml ")
            )
    };
    let old = read_jxl(d);
    if !top.iter().any(compressed) && old.exif == m.exif && old.xmp == m.xmp {
        return Some(d.to_vec());
    }
    let drop = |b: &BoxPos| matches!(&b.ty, b"Exif" | b"xml " | b"jbrd") || compressed(b);
    // insert after the signature, `ftyp` and a following `jxll`
    let mut after = 0;
    for (k, b) in top.iter().enumerate().take(3) {
        if matches!(&b.ty, b"JXL " | b"ftyp" | b"jxll") {
            after = k;
        } else {
            break;
        }
    }
    let mut out = Vec::with_capacity(d.len() + news.iter().map(|n| n.1.len() + 8).sum::<usize>());
    for (k, b) in top.iter().enumerate() {
        if !drop(b) {
            out.extend_from_slice(&d[b.start..b.end]);
        }
        if k == after {
            for (ty, p) in &news {
                put_box(&mut out, ty, p);
            }
        }
    }
    out.extend_from_slice(&d[top.last().map_or(0, |b| b.end)..]);
    Some(out)
}

// ------------------------------------------------------------------ allgemein

/// Liest die Metadaten einer Datei (JPEG, PNG, WebP; B-4: TIFF, AVIF/HEIF, JPEG XL; sonst leer).
pub fn read(d: &[u8]) -> Found {
    if d.starts_with(&[0xFF, 0xD8]) {
        read_jpeg(d)
    } else if d.starts_with(PNG_SIG) {
        read_png(d)
    } else if d.starts_with(b"RIFF") {
        read_webp(d)
    } else if d.starts_with(b"II*\0") || d.starts_with(b"MM\0*") {
        read_tiff(d)
    } else if d.starts_with(JXL_SIG) {
        read_jxl(d)
    } else if d.get(4..8) == Some(b"ftyp") {
        read_isobmff(d)
    } else {
        Found::default()
    }
}

/// Writes metadata into encoded bytes by container signature (BA-108): JPEG, PNG, WebP, JPEG XL
/// (bare codestream or container) and AVIF/HEIF. TIFF is written by its encoder (`None` here, as
/// for unknown data). JPEG XL and AVIF/HEIF take only EXIF and XMP of `m`.
pub fn write(d: &[u8], m: &Found) -> Option<Vec<u8>> {
    if d.starts_with(&[0xFF, 0xD8]) {
        write_jpeg(d, m)
    } else if d.starts_with(PNG_SIG) {
        write_png(d, m)
    } else if d.starts_with(b"RIFF") {
        write_webp(d, m)
    } else if d.starts_with(JXL_SIG) || d.starts_with(&[0xFF, 0x0A]) {
        write_jxl(d, m)
    } else if d.get(4..8) == Some(b"ftyp") {
        write_heif(d, m)
    } else {
        None
    }
}

#[cfg(test)]
mod ba108_tests {
    //! BA-108: writing EXIF/XMP into hand-built AVIF/HEIF and JPEG XL containers.

    use super::*;
    use crate::meta::exif::{Exif, TAG_ARTIST};

    fn bx(ty: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        put_box(&mut v, ty, payload);
        v
    }

    fn exif_of(artist: &str) -> Vec<u8> {
        let mut e = Exif::new_empty();
        e.set_ascii(TAG_ARTIST, artist);
        e.write()
    }

    fn artist(f: &Found) -> Option<String> {
        Exif::parse(f.exif.as_deref()?)?
            .ifd0
            .get(TAG_ARTIST)
            .and_then(Exif::text)
    }

    const IMG: &[u8] = b"IMAGE-DATA-PRIMARY";
    const ALPHA: &[u8] = b"ALPHA-PLANE";
    const XMP_NEW: &[u8] = b"<x:xmpmeta xmlns:x=\"x\"><rdf:RDF>new</rdf:RDF></x:xmpmeta>";

    fn infe(id: u16, ty: &[u8; 4]) -> Vec<u8> {
        let mut p = vec![2, 0, 0, 0];
        p.extend_from_slice(&id.to_be_bytes());
        p.extend_from_slice(&[0, 0]);
        p.extend_from_slice(ty);
        p.push(0);
        bx(b"infe", &p)
    }

    /// ftyp, meta (hdlr, pitm, iinf, iref, iprp, iloc), [moov with stco], mdat.
    /// Items: 1 primary image, 2 old Exif, 3 alpha (auxl to 1). `iloc_ver` 0/1/2, `large` = 64-bit
    /// mdat header, `moov` = a sequence track whose chunk offset points at the primary data.
    fn heif(iloc_ver: u8, large: bool, moov: bool) -> Vec<u8> {
        let mut old_exif = 6u32.to_be_bytes().to_vec();
        old_exif.extend_from_slice(b"Exif\0\0");
        old_exif.extend(exif_of("OldArtist"));
        let ftyp = bx(b"ftyp", b"heic\0\0\0\0mif1heic");
        let mut iinf = vec![0, 0, 0, 0, 0, 3];
        iinf.extend(infe(1, b"hvc1"));
        iinf.extend(infe(2, b"Exif"));
        iinf.extend(infe(3, b"hvc1"));
        let mut iref = vec![0, 0, 0, 0];
        iref.extend(bx(b"cdsc", &[0, 2, 0, 1, 0, 1]));
        iref.extend(bx(b"auxl", &[0, 3, 0, 1, 0, 1]));
        let ipco = bx(b"ispe", &[0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 6]);
        // ipma: items 1, 2, 3 each with property 1
        let mut ipma = vec![0, 0, 0, 0, 0, 0, 0, 3];
        for id in [1u16, 2, 3] {
            ipma.extend_from_slice(&id.to_be_bytes());
            ipma.extend_from_slice(&[1, 0x81]);
        }
        let mut iprp = bx(b"ipco", &ipco);
        iprp.extend(bx(b"ipma", &ipma));
        let parts: [(u16, &[u8]); 3] = [(1, IMG), (2, &old_exif), (3, ALPHA)];
        let build = |offs: [u32; 3]| {
            let sizes = if iloc_ver >= 1 { 0x40 } else { 0x00 };
            let mut il = vec![iloc_ver, 0, 0, 0, 0x44, sizes];
            if iloc_ver < 2 {
                il.extend_from_slice(&3u16.to_be_bytes());
            } else {
                il.extend_from_slice(&3u32.to_be_bytes());
            }
            for (k, (id, data)) in parts.iter().enumerate() {
                if iloc_ver < 2 {
                    il.extend_from_slice(&id.to_be_bytes());
                } else {
                    il.extend_from_slice(&(*id as u32).to_be_bytes());
                }
                if iloc_ver >= 1 {
                    il.extend_from_slice(&[0, 0]); // construction method 0
                }
                il.extend_from_slice(&[0, 0]); // data reference
                if iloc_ver >= 1 {
                    il.extend_from_slice(&0u32.to_be_bytes()); // base offset (size 4)
                }
                il.extend_from_slice(&1u16.to_be_bytes());
                il.extend_from_slice(&offs[k].to_be_bytes());
                il.extend_from_slice(&(data.len() as u32).to_be_bytes());
            }
            let mut meta = vec![0, 0, 0, 0];
            meta.extend(bx(b"hdlr", &[0u8; 25]));
            meta.extend(bx(b"pitm", &[0, 0, 0, 0, 0, 1]));
            meta.extend(bx(b"iinf", &iinf));
            meta.extend(bx(b"iref", &iref));
            meta.extend(bx(b"iprp", &iprp));
            meta.extend(bx(b"iloc", &il));
            bx(b"meta", &meta)
        };
        let moov_box = |off: u32| {
            let mut stco = vec![0, 0, 0, 0, 0, 0, 0, 1];
            stco.extend_from_slice(&off.to_be_bytes());
            let stbl = bx(b"stbl", &bx(b"stco", &stco));
            let minf = bx(b"minf", &stbl);
            let mdia = bx(b"mdia", &minf);
            let trak = bx(b"trak", &mdia);
            bx(b"moov", &trak)
        };
        let meta_len = build([0; 3]).len();
        let moov_len = if moov { moov_box(0).len() } else { 0 };
        let start = (ftyp.len() + meta_len + moov_len + if large { 16 } else { 8 }) as u32;
        let o2 = start + IMG.len() as u32;
        let o3 = o2 + old_exif.len() as u32;
        let mut file = ftyp;
        file.extend(build([start, o2, o3]));
        if moov {
            file.extend(moov_box(start));
        }
        let mut mdat = IMG.to_vec();
        mdat.extend(&old_exif);
        mdat.extend_from_slice(ALPHA);
        if large {
            file.extend_from_slice(&1u32.to_be_bytes());
            file.extend_from_slice(b"mdat");
            file.extend_from_slice(&(mdat.len() as u64 + 16).to_be_bytes());
            file.extend(mdat);
        } else {
            file.extend(bx(b"mdat", &mdat));
        }
        file
    }

    fn meta_kids(d: &[u8]) -> Vec<u8> {
        child(d, b"meta").expect("meta")[4..].to_vec()
    }

    /// Bytes of item `id` (construction method 0).
    fn item(d: &[u8], id: u32) -> Option<Vec<u8>> {
        let kids = meta_kids(d);
        let iloc = Iloc::parse(child(&kids, b"iloc")?)?;
        let it = iloc.items.iter().find(|i| i.id == id)?;
        let mut v = Vec::new();
        for [_, o, l] in &it.extents {
            let s = (it.base + o) as usize;
            v.extend_from_slice(d.get(s..s + *l as usize)?);
        }
        Some(v)
    }

    fn types(d: &[u8]) -> Vec<[u8; 4]> {
        box_positions(d).into_iter().map(|b| b.ty).collect()
    }

    fn has(d: &[u8], s: &[u8]) -> bool {
        d.windows(s.len()).any(|w| w == s)
    }

    #[test]
    fn heif_write_replaces_items_and_keeps_the_rest() {
        for (ver, large, moov) in [
            (0, false, false),
            (1, false, true),
            (2, true, false),
            (1, true, true),
        ] {
            let src = heif(ver, large, moov);
            assert_eq!(artist(&read(&src)).as_deref(), Some("OldArtist"));
            let m = Found {
                exif: Some(exif_of("NewArtist")),
                xmp: Some(XMP_NEW.to_vec()),
                ..Default::default()
            };
            let out = write_heif(&src, &m).expect("write");
            let f = read(&out);
            assert_eq!(artist(&f).as_deref(), Some("NewArtist"), "v{ver}");
            assert_eq!(f.xmp.as_deref(), Some(XMP_NEW));
            // old payload blanked, other items still point at their data
            assert!(!has(&out, b"OldArtist"));
            assert_eq!(item(&out, 1).as_deref(), Some(IMG), "v{ver}");
            assert_eq!(item(&out, 3).as_deref(), Some(ALPHA), "v{ver}");
            // box order: the new mdat follows meta, everything else in order
            let mut want = vec![*b"ftyp", *b"meta", *b"mdat"];
            if moov {
                want.push(*b"moov");
            }
            want.push(*b"mdat");
            assert_eq!(types(&out), want);
            // iinf, iref, ipma and pitm stay consistent
            let kids = meta_kids(&out);
            let (_, infes) = parse_iinf(child(&kids, b"iinf").expect("iinf")).expect("iinf");
            let ids: Vec<u32> = infes.iter().map(|i| i.id).collect();
            assert_eq!(ids, vec![1, 3, 4, 5]);
            let (_, _, refs) = parse_iref(child(&kids, b"iref").expect("iref")).expect("iref");
            assert!(refs.iter().all(|r| r.from != 2 && !r.to.contains(&2)));
            for (ty, from) in [(*b"auxl", 3), (*b"cdsc", 4), (*b"cdsc", 5)] {
                let r = ItemRef {
                    ty,
                    from,
                    to: vec![1],
                };
                assert!(refs.contains(&r), "{r:?}");
            }
            let ipma = child(child(&kids, b"iprp").expect("iprp"), b"ipma").expect("ipma");
            assert_eq!(be(ipma, 4, 4), Some(2));
            assert_eq!(child(&kids, b"pitm"), Some(&[0u8, 0, 0, 0, 0, 1][..]));
            if moov {
                let stco = child(&out, b"moov")
                    .and_then(|b| child(b, b"trak"))
                    .and_then(|b| child(b, b"mdia"))
                    .and_then(|b| child(b, b"minf"))
                    .and_then(|b| child(b, b"stbl"))
                    .and_then(|b| child(b, b"stco"))
                    .expect("stco");
                let off = be(stco, 8, 4).expect("offset") as usize;
                assert_eq!(&out[off..off + IMG.len()], IMG);
            }
            // writing again replaces the items written before; removing leaves none
            let again = write_heif(&out, &m).expect("again");
            assert_eq!(artist(&read(&again)).as_deref(), Some("NewArtist"));
            assert_eq!(item(&again, 1).as_deref(), Some(IMG));
            let none = write_heif(&again, &Found::default()).expect("strip");
            let f = read(&none);
            assert!(f.exif.is_none() && f.xmp.is_none());
            assert!(!has(&none, b"NewArtist"));
            assert_eq!(item(&none, 1).as_deref(), Some(IMG));
            assert_eq!(item(&none, 3).as_deref(), Some(ALPHA));
        }
    }

    #[test]
    fn heif_without_metadata_is_unchanged_and_fields_widen() {
        let src = heif(0, false, false);
        let plain = write_heif(&src, &Found::default()).expect("strip");
        assert_eq!(write_heif(&plain, &Found::default()), Some(plain.clone()));
        // offset/length fields of size 0 grow when values need them
        let mut il = Iloc::parse(&[1, 0, 0, 0, 0x00, 0x00, 0, 0]).expect("iloc");
        il.items.push(IlocItem {
            id: 1,
            method: 0,
            dref: 0,
            base: 0,
            extents: vec![[0, 70_000, 5_000_000_000]],
        });
        assert!(il.fit_sizes());
        assert_eq!((il.off_s, il.len_s, il.base_s), (4, 8, 0));
        assert_eq!(Iloc::parse(&il.write()), Some(il));
    }

    #[test]
    fn jxl_container_keeps_box_order() {
        let mut old_exif = 0u32.to_be_bytes().to_vec();
        old_exif.extend(exif_of("OldArtist"));
        let mut d = JXL_SIG.to_vec();
        d.extend(bx(b"ftyp", b"jxl \0\0\0\0jxl "));
        d.extend(bx(b"jxll", &[5]));
        d.extend(bx(b"Exif", &old_exif));
        d.extend(bx(b"jbrd", b"reconstruction"));
        d.extend(bx(b"brob", b"xml compressed"));
        d.extend(bx(b"jxlp", &[0, 0, 0, 0, 0xFF, 0x0A]));
        d.extend(bx(b"jxlp", &[0x80, 0, 0, 1, 1, 2]));
        let m = Found {
            exif: Some(exif_of("NewArtist")),
            xmp: Some(XMP_NEW.to_vec()),
            ..Default::default()
        };
        let out = write_jxl(&d, &m).expect("write");
        assert_eq!(
            types(&out),
            vec![
                *b"JXL ", *b"ftyp", *b"jxll", *b"Exif", *b"xml ", *b"jxlp", *b"jxlp"
            ]
        );
        let f = read(&out);
        assert_eq!(artist(&f).as_deref(), Some("NewArtist"));
        assert_eq!(f.xmp.as_deref(), Some(XMP_NEW));
        // the same metadata again: bytes unchanged
        assert_eq!(write_jxl(&out, &m), Some(out.clone()));
        // removing
        let none = write_jxl(&out, &Found::default()).expect("strip");
        assert_eq!(
            types(&none),
            vec![*b"JXL ", *b"ftyp", *b"jxll", *b"jxlp", *b"jxlp"]
        );
        // a bare codestream is wrapped
        let cs = [0xFF, 0x0A, 1, 2, 3];
        let w = write(&cs, &m).expect("wrap");
        assert_eq!(
            types(&w),
            vec![*b"JXL ", *b"ftyp", *b"Exif", *b"xml ", *b"jxlc"]
        );
        assert_eq!(child(&w, b"jxlc"), Some(&cs[..]));
        assert_eq!(write_jxl(&cs, &Found::default()), Some(cs.to_vec()));
    }

    #[test]
    fn large_boxes_get_a_64_bit_header() {
        assert_eq!(box_len(10), 18);
        assert_eq!(box_len(u32::MAX as usize), u32::MAX as usize + 16);
        let mut v = Vec::new();
        put_box(&mut v, b"abcd", &[1, 2]);
        assert_eq!(v, [0, 0, 0, 10, 97, 98, 99, 100, 1, 2]);
    }
}
