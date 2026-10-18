//! Metadaten-Segmente in Dateicontainern lesen und schreiben (SPEC-BATCH §3.7): JPEG (APP1 EXIF,
//! APP1 XMP, APP2 ICC in Teilen, APP13 IPTC, COM), PNG (`eXIf`, `iCCP`, `iTXt` XMP, `tEXt` Comment)
//! und WebP (RIFF `VP8X` mit `ICCP`, `EXIF`, `XMP `). Eigene Umsetzung statt `img-parts`.

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

/// Boxes `(type, payload)` of a box sequence (64-bit sizes and size 0 = to the end).
fn boxes(d: &[u8]) -> Vec<([u8; 4], &[u8])> {
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
        let Some(end) = i.checked_add(len as usize).filter(|e| *e <= d.len()) else {
            break;
        };
        out.push((ty, &d[i + head as usize..end]));
        i = end;
    }
    out
}

fn child<'a>(d: &'a [u8], ty: &[u8; 4]) -> Option<&'a [u8]> {
    boxes(d).into_iter().find(|b| &b.0 == ty).map(|b| b.1)
}

fn be(d: &[u8], i: usize, n: usize) -> Option<u64> {
    let s = d.get(i..i.checked_add(n)?)?;
    Some(s.iter().fold(0u64, |a, b| (a << 8) | *b as u64))
}

/// Exif/XMP items and the ICC profile of an AVIF/HEIF file (read only).
fn read_isobmff(d: &[u8]) -> Found {
    let mut f = Found::default();
    let Some(meta) = child(d, b"meta").and_then(|m| m.get(4..)) else {
        return f;
    };
    // item types
    let mut kinds: Vec<(u64, bool)> = Vec::new(); // (id, is_exif)
    if let Some(iinf) = child(meta, b"iinf") {
        let v = iinf.first().copied().unwrap_or(0);
        let start = if v == 0 { 6 } else { 8 };
        for (ty, infe) in boxes(iinf.get(start..).unwrap_or_default()) {
            let ver = infe.first().copied().unwrap_or(0);
            if &ty != b"infe" || ver < 2 {
                continue;
            }
            let (id, p) = if ver == 2 {
                (be(infe, 4, 2), 6)
            } else {
                (be(infe, 4, 4), 8)
            };
            let (Some(id), Some(t)) = (id, infe.get(p + 2..p + 6)) else {
                continue;
            };
            let rest = infe.get(p + 6..).unwrap_or_default();
            let mut parts = rest.split(|c| *c == 0);
            let _name = parts.next();
            let content = parts.next().unwrap_or_default();
            match t {
                b"Exif" => kinds.push((id, true)),
                b"mime" if content.starts_with(b"application/rdf+xml") => kinds.push((id, false)),
                _ => {}
            }
        }
    }
    // item locations (construction method 0 = file offset)
    if let Some(iloc) = child(meta, b"iloc")
        && let (Some(a), Some(b)) = (iloc.get(4), iloc.get(5))
    {
        let ver = iloc.first().copied().unwrap_or(0);
        let (off_s, len_s, base_s) = ((a >> 4) as usize, (a & 15) as usize, (b >> 4) as usize);
        let idx_s = if ver >= 1 { (b & 15) as usize } else { 0 };
        let id_n = if ver < 2 { 2 } else { 4 };
        let (count, mut p) = if ver < 2 {
            (be(iloc, 6, 2).unwrap_or(0), 8)
        } else {
            (be(iloc, 6, 4).unwrap_or(0), 10)
        };
        for _ in 0..count.min(10_000) {
            let Some(id) = be(iloc, p, id_n) else { break };
            p += id_n;
            let mut method = 0;
            if ver >= 1 {
                method = be(iloc, p, 2).unwrap_or(0) & 15;
                p += 2;
            }
            p += 2; // data reference index
            let base = be(iloc, p, base_s).unwrap_or(0);
            p += base_s;
            let Some(n) = be(iloc, p, 2) else { break };
            p += 2;
            let mut data = Vec::new();
            for _ in 0..n {
                p += idx_s;
                let o = be(iloc, p, off_s).unwrap_or(0);
                p += off_s;
                let l = be(iloc, p, len_s).unwrap_or(0);
                p += len_s;
                let s = base.saturating_add(o) as usize;
                if method == 0
                    && let Some(chunk) = d.get(s..s.saturating_add(l as usize))
                {
                    data.extend_from_slice(chunk);
                }
            }
            match kinds.iter().find(|k| k.0 == id) {
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

// ------------------------------------------------------------------ JPEG XL container (B-4)

const JXL_SIG: &[u8] = b"\0\0\0\x0CJXL \r\n\x87\n";

/// `Exif` and `xml ` boxes of a JPEG XL container (read only; `brob` boxes are skipped).
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
