//! Eigene Kodierer für „Speichern als“ (SPEC-BATCH §3.2): TIFF mit Kompression none/lzw/deflate/
//! packbits (Vorgabe lzw, Prädiktor 2), ICC-Profil und XMP als Tags; TGA (RLE oder unkomprimiert,
//! 24/32 Bit, Ursprung oben links). `fp-export` kann bei TIFF nur none/packbits und hat kein TGA.

use crate::error::{BatchError, Result};
use crate::item::{Item, Meta};
use fp_raster::RgbaImage;
use serde_json::{Map, Value};
use std::io::Write;

/// Bild für Formate ohne Animation: Einzelbild, erstes Frame bzw. flach gerechnetes Dokument.
pub fn first_image(item: &Item) -> Result<RgbaImage> {
    item.preview_image()
}

// ------------------------------------------------------------------ TIFF

/// TIFF-Kompression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TiffCompression {
    None,
    Lzw,
    Deflate,
    PackBits,
}

impl TiffCompression {
    pub fn parse(s: Option<&str>) -> TiffCompression {
        match s {
            Some("none") => TiffCompression::None,
            Some("deflate" | "zip") => TiffCompression::Deflate,
            Some("packbits") => TiffCompression::PackBits,
            _ => TiffCompression::Lzw,
        }
    }
    fn code(self) -> u16 {
        match self {
            TiffCompression::None => 1,
            TiffCompression::Lzw => 5,
            TiffCompression::Deflate => 8,
            TiffCompression::PackBits => 32773,
        }
    }
}

struct BitWriter {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl BitWriter {
    fn put(&mut self, code: u16, width: u32) {
        self.acc = (self.acc << width) | code as u32;
        self.n += width;
        while self.n >= 8 {
            self.n -= 8;
            self.out.push((self.acc >> self.n) as u8);
            self.acc &= (1 << self.n) - 1;
        }
    }
    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.out.push((self.acc << (8 - self.n)) as u8);
        }
        self.out
    }
}

/// LZW nach TIFF 6.0 (MSB zuerst, „early change“).
pub fn lzw_encode(data: &[u8]) -> Vec<u8> {
    use std::collections::HashMap;
    const CLEAR: u16 = 256;
    const EOI: u16 = 257;
    let mut w = BitWriter {
        out: Vec::with_capacity(data.len() / 2),
        acc: 0,
        n: 0,
    };
    let mut dict: HashMap<(u16, u8), u16> = HashMap::new();
    let mut next: u16 = 258;
    let mut width = 9u32;
    w.put(CLEAR, width);
    let mut cur: Option<u16> = None;
    for &b in data {
        let Some(c) = cur else {
            cur = Some(b as u16);
            continue;
        };
        if let Some(&k) = dict.get(&(c, b)) {
            cur = Some(k);
            continue;
        }
        w.put(c, width);
        dict.insert((c, b), next);
        next += 1;
        if next as u32 == (1 << width) && width < 12 {
            width += 1;
        }
        if next >= 4094 {
            w.put(CLEAR, width);
            dict.clear();
            next = 258;
            width = 9;
        }
        cur = Some(b as u16);
    }
    if let Some(c) = cur {
        w.put(c, width);
        next += 1;
        if next as u32 == (1 << width) && width < 12 {
            width += 1;
        }
    }
    w.put(EOI, width);
    w.finish()
}

fn rational(v: f32) -> Vec<u8> {
    let num = (v.max(1.0) * 100.0).round() as u32;
    let mut b = num.to_le_bytes().to_vec();
    b.extend_from_slice(&100u32.to_le_bytes());
    b
}

/// Schreibt ein TIFF (Little Endian, ein Streifen, 8 Bit je Kanal).
pub fn write_tiff(img: &RgbaImage, meta: &Meta, compression: TiffCompression) -> Result<Vec<u8>> {
    write_tiff_pages(std::slice::from_ref(img), meta, compression)
}

/// Multi-page TIFF (B-4): one IFD per page, chained through the next-IFD offset. Each page has its
/// own size, one strip and its own alpha decision; with more than one page every IFD also carries
/// `NewSubfileType = 2` (page) and `PageNumber`. ICC/XMP go into every IFD. One page gives exactly
/// the bytes of the single-page writer.
pub fn write_tiff_pages(
    pages: &[RgbaImage],
    meta: &Meta,
    compression: TiffCompression,
) -> Result<Vec<u8>> {
    if pages.is_empty() {
        return Err(BatchError::Encode(fp_i18n::t("batch-err-empty-page")));
    }
    let mut out = b"II".to_vec();
    out.extend_from_slice(&42u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    let mut next_ptr = 4usize;
    let total = pages.len();
    for (i, img) in pages.iter().enumerate() {
        let page = (total > 1).then_some((i as u16, total.min(u16::MAX as usize) as u16));
        let ptr = tiff_page(&mut out, next_ptr, img, meta, compression, page)?;
        next_ptr = ptr;
    }
    Ok(out)
}

/// Appends one page (strip, IFD, IFD data) to `out`, writes the IFD offset at `link_at` and
/// returns the position of this IFD's next-IFD offset.
fn tiff_page(
    out: &mut Vec<u8>,
    link_at: usize,
    img: &RgbaImage,
    meta: &Meta,
    compression: TiffCompression,
    page: Option<(u16, u16)>,
) -> Result<usize> {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 || w > u32::MAX as usize / 4 {
        return Err(BatchError::Encode(fp_i18n::t("batch-err-empty-page")));
    }
    let alpha = img.data().iter().any(|p| p[3] < 255);
    let spp = if alpha { 4 } else { 3 };
    let predictor = matches!(compression, TiffCompression::Lzw | TiffCompression::Deflate);
    let mut raw = Vec::with_capacity(w * h * spp);
    for y in 0..h {
        let row = img.row_local(y);
        let start = raw.len();
        for p in row {
            raw.extend_from_slice(&p[..spp]);
        }
        if predictor {
            for i in (start + spp..raw.len()).rev() {
                raw[i] = raw[i].wrapping_sub(raw[i - spp]);
            }
        }
    }
    let strip = match compression {
        TiffCompression::None => raw,
        TiffCompression::Lzw => lzw_encode(&raw),
        TiffCompression::Deflate => {
            let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            e.write_all(&raw)
                .map_err(|e| BatchError::Encode(e.to_string()))?;
            e.finish().map_err(|e| BatchError::Encode(e.to_string()))?
        }
        TiffCompression::PackBits => raw
            .chunks(w * spp)
            .flat_map(fp_export::formats::packbits)
            .collect(),
    };
    let dpi = meta.dpi.unwrap_or(72.0);
    let s16 = |v: u16| v.to_le_bytes().to_vec();
    let l32 = |v: u32| v.to_le_bytes().to_vec();
    // (Tag, Typ, Anzahl, Wert)
    let mut tags: Vec<(u16, u16, u32, Vec<u8>)> = vec![
        (256, 4, 1, l32(w as u32)),
        (257, 4, 1, l32(h as u32)),
        (
            258,
            3,
            spp as u32,
            (0..spp).flat_map(|_| 8u16.to_le_bytes()).collect(),
        ),
        (259, 3, 1, s16(compression.code())),
        (262, 3, 1, s16(2)),
        (273, 4, 1, l32(0)),
        (277, 3, 1, s16(spp as u16)),
        (278, 4, 1, l32(h as u32)),
        (279, 4, 1, l32(strip.len() as u32)),
        (282, 5, 1, rational(dpi)),
        (283, 5, 1, rational(dpi)),
        (284, 3, 1, s16(1)),
        (296, 3, 1, s16(2)),
    ];
    if let Some((n, total)) = page {
        tags.push((254, 4, 1, l32(2)));
        tags.push((297, 3, 2, [s16(n), s16(total)].concat()));
    }
    if predictor {
        tags.push((317, 3, 1, s16(2)));
    }
    if alpha {
        tags.push((338, 3, 1, s16(2)));
    }
    if let Some(x) = &meta.xmp {
        tags.push((700, 1, x.len() as u32, x.clone()));
    }
    if let Some(icc) = &meta.icc {
        tags.push((34675, 7, icc.len() as u32, icc.clone()));
    }
    // B-4: IPTC (raw IIM) and EXIF (descriptive IFD0 tags, EXIF and GPS sub-IFDs)
    if let Some(iim) = super::iptc(meta).and_then(|irb| super::iptc::iim_of(&irb)) {
        tags.push((33723, 7, iim.len() as u32, iim));
    }
    let mut subs: Vec<(u16, Vec<u8>)> = Vec::new();
    if let Some(ex) = meta.exif.as_deref().and_then(super::exif::Exif::parse) {
        let le = ex.little_endian;
        for e in &ex.ifd0.entries {
            if TIFF_EXIF_IFD0_TAGS.contains(&e.tag) && !tags.iter().any(|t| t.0 == e.tag) {
                tags.push((e.tag, e.ty, e.count, entry_le(e, le)));
            }
        }
        for (tag, ifd) in [(34665u16, &ex.exif), (34853, &ex.gps)] {
            if let Some(ifd) = ifd.as_ref().filter(|i| !i.entries.is_empty()) {
                let entries: Vec<super::exif::Entry> = ifd
                    .entries
                    .iter()
                    .filter(|e| e.tag != super::exif::TAG_INTEROP_IFD)
                    .map(|e| super::exif::Entry {
                        data: entry_le(e, le),
                        ..e.clone()
                    })
                    .collect();
                // placeholder pointer; the IFD is placed with the blobs
                tags.push((tag, 4, 1, l32(0)));
                subs.push((tag, entries_to_bytes(&entries)));
            }
        }
    }
    tags.sort_by_key(|t| t.0);
    if out.len() % 2 == 1 {
        out.push(0);
    }
    let strip_at = out.len() as u32;
    out.extend_from_slice(&strip);
    if out.len() % 2 == 1 {
        out.push(0);
    }
    let ifd_at = out.len();
    out[link_at..link_at + 4].copy_from_slice(&(ifd_at as u32).to_le_bytes());
    let n = tags.len();
    let next_at = ifd_at + 2 + 12 * n;
    let mut data_at = next_at + 4;
    let mut blobs = Vec::new();
    out.extend_from_slice(&(n as u16).to_le_bytes());
    for (tag, ty, count, mut val) in tags {
        if tag == 273 {
            val = l32(strip_at);
        }
        if let Some(i) = subs.iter().position(|s| s.0 == tag) {
            // B-4: sub-IFD at the next even offset, values right behind it
            data_at += data_at % 2;
            let entries = sub_entries(&subs[i].1);
            let bytes = sub_ifd(&entries, data_at);
            out.extend_from_slice(&tag.to_le_bytes());
            out.extend_from_slice(&ty.to_le_bytes());
            out.extend_from_slice(&count.to_le_bytes());
            out.extend_from_slice(&(data_at as u32).to_le_bytes());
            let len = bytes.len();
            blobs.push((data_at, bytes));
            data_at += len;
            continue;
        }
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&ty.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        if val.len() <= 4 {
            val.resize(4, 0);
            out.extend_from_slice(&val);
        } else {
            data_at += data_at % 2;
            out.extend_from_slice(&(data_at as u32).to_le_bytes());
            let len = val.len();
            blobs.push((data_at, val));
            data_at += len;
        }
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    for (at, v) in blobs {
        out.resize(at, 0);
        out.extend_from_slice(&v);
    }
    Ok(next_at)
}

/// IFD0 tags of an EXIF block that are copied into a TIFF IFD (description, make, model,
/// software, date, artist, copyright, XP tags). Orientation is not copied: pixels are written
/// already oriented.
pub const TIFF_EXIF_IFD0_TAGS: [u16; 12] = [
    0x010E, 0x010F, 0x0110, 0x0131, 0x0132, 0x013B, 0x8298, 0x9C9B, 0x9C9C, 0x9C9D, 0x9C9E, 0x9C9F,
];

/// Entry data in little-endian order (`src_le` = byte order of the source block).
pub fn entry_le(e: &super::exif::Entry, src_le: bool) -> Vec<u8> {
    if src_le {
        return e.data.clone();
    }
    let unit = match e.ty {
        3 | 8 => 2,
        4 | 9 | 11 | 13 | 5 | 10 => 4,
        12 => 8,
        _ => 1,
    };
    let mut d = e.data.clone();
    if unit > 1 {
        for c in d.chunks_mut(unit) {
            c.reverse();
        }
    }
    d
}

/// Serialised entries for the transport between the passes (tag, type, count, len, data).
fn entries_to_bytes(entries: &[super::exif::Entry]) -> Vec<u8> {
    let mut o = Vec::new();
    for e in entries {
        o.extend_from_slice(&e.tag.to_le_bytes());
        o.extend_from_slice(&e.ty.to_le_bytes());
        o.extend_from_slice(&e.count.to_le_bytes());
        o.extend_from_slice(&(e.data.len() as u32).to_le_bytes());
        o.extend_from_slice(&e.data);
    }
    o
}

fn sub_entries(b: &[u8]) -> Vec<super::exif::Entry> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 12 <= b.len() {
        let tag = u16::from_le_bytes([b[i], b[i + 1]]);
        let ty = u16::from_le_bytes([b[i + 2], b[i + 3]]);
        let count = u32::from_le_bytes([b[i + 4], b[i + 5], b[i + 6], b[i + 7]]);
        let n = u32::from_le_bytes([b[i + 8], b[i + 9], b[i + 10], b[i + 11]]) as usize;
        let Some(data) = b.get(i + 12..i + 12 + n) else {
            break;
        };
        out.push(super::exif::Entry {
            tag,
            ty,
            count,
            data: data.to_vec(),
        });
        i += 12 + n;
    }
    out
}

/// Little-endian IFD placed at absolute offset `at` (values > 4 bytes follow the IFD).
fn sub_ifd(entries: &[super::exif::Entry], at: usize) -> Vec<u8> {
    let mut list: Vec<&super::exif::Entry> = entries.iter().collect();
    list.sort_by_key(|e| e.tag);
    let n = list.len();
    let mut o = Vec::new();
    o.extend_from_slice(&(n as u16).to_le_bytes());
    let data_at = at + 2 + 12 * n + 4;
    let mut tail: Vec<u8> = Vec::new();
    for e in list {
        o.extend_from_slice(&e.tag.to_le_bytes());
        o.extend_from_slice(&e.ty.to_le_bytes());
        o.extend_from_slice(&e.count.to_le_bytes());
        if e.data.len() <= 4 {
            let mut v = e.data.clone();
            v.resize(4, 0);
            o.extend_from_slice(&v);
        } else {
            o.extend_from_slice(&((data_at + tail.len()) as u32).to_le_bytes());
            tail.extend_from_slice(&e.data);
            if tail.len() % 2 == 1 {
                tail.push(0);
            }
        }
    }
    o.extend_from_slice(&0u32.to_le_bytes());
    o.extend_from_slice(&tail);
    o
}

// ------------------------------------------------------------------ TGA

/// Schreibt ein TGA (Typ 10 RLE bzw. 2 unkomprimiert; 32 Bit mit Alpha, sonst 24 Bit).
pub fn write_tga(img: &RgbaImage, rle: bool) -> Result<Vec<u8>> {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 || w > 65535 || h > 65535 {
        return Err(BatchError::Encode(fp_i18n::tr!(
            "batch-meta-err-size",
            format = "TGA",
            max = 65535
        )));
    }
    let alpha = img.data().iter().any(|p| p[3] < 255);
    let bpp = if alpha { 4 } else { 3 };
    let mut out = vec![0u8; 18];
    out[2] = if rle { 10 } else { 2 };
    out[12..14].copy_from_slice(&(w as u16).to_le_bytes());
    out[14..16].copy_from_slice(&(h as u16).to_le_bytes());
    out[16] = (bpp * 8) as u8;
    out[17] = 0x20 | if alpha { 8 } else { 0 };
    let px = |p: &[u8; 4]| -> [u8; 4] { [p[2], p[1], p[0], p[3]] };
    for y in 0..h {
        let row = img.row_local(y);
        if !rle {
            for p in row {
                out.extend_from_slice(&px(p)[..bpp]);
            }
            continue;
        }
        let mut i = 0;
        while i < w {
            let mut run = 1;
            while i + run < w && run < 128 && row[i + run] == row[i] {
                run += 1;
            }
            if run >= 2 {
                out.push(0x80 | (run - 1) as u8);
                out.extend_from_slice(&px(&row[i])[..bpp]);
                i += run;
                continue;
            }
            let start = i;
            let mut lit = 0;
            while i < w && lit < 128 && !(i + 1 < w && row[i] == row[i + 1]) {
                i += 1;
                lit += 1;
            }
            if lit == 0 {
                i += 1;
                lit = 1;
            }
            out.push((lit - 1) as u8);
            for p in &row[start..start + lit] {
                out.extend_from_slice(&px(p)[..bpp]);
            }
        }
    }
    // TGA-2.0-Fußzeile
    out.extend_from_slice(&[0; 8]);
    out.extend_from_slice(b"TRUEVISION-XFILE.\0");
    Ok(out)
}

/// Eingehängter Kodierer für TGA (`output::register_encoder`). Option `compression`: `none` → unkomprimiert.
pub fn encode_tga(item: &Item, options: &Map<String, Value>) -> Result<Vec<u8>> {
    let rle = options.get("compression").and_then(Value::as_str) != Some("none");
    write_tga(&first_image(item)?, rle)
}
