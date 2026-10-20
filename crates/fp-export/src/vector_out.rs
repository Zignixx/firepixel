//! Einfache Dokument-Exporte: PDF (je Seite ein Bild) und SVG (Pfade als Vektoren, Bitmaps und
//! übrige Objekte als eingebettete Bilder).
//!
//! Das Original schreibt PDF über die PDF-Bibliothek (08 §5.4, A) und kennt keinen SVG-Export (B). Firepixel bietet
//! beides in schlichter Form: PDF-Seiten enthalten das flach gerechnete Seitenbild (Flate, Alpha als
//! SMask) in der Seitengröße nach der Dokumentauflösung; SVG enthält Pfade mit Vollton-Füllung/-Strich als
//! `<path>`, Bitmaps als `<image>` (PNG, Base64) und — falls ein Seitenbild übergeben wird — dieses als
//! unterste Ebene für alles, was nicht als Vektor darstellbar ist.

use std::io::Write;

use fp_model::{Document, Element, ElementKind, LayerKind, LayerSlot};
use fp_raster::RgbaImage;

use crate::png_out::{PngOptions, write_png_truecolor};
use crate::px::from_rgba;
use crate::{ExportError, Result};

fn deflate(d: &[u8]) -> Result<Vec<u8>> {
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(6));
    e.write_all(d)?;
    Ok(e.finish()?)
}

/// Eine PDF-Seite: Bild und Größe in Punkt.
pub struct PdfPage<'a> {
    pub image: &'a RgbaImage,
    pub width_pt: f64,
    pub height_pt: f64,
    /// Invisible text over the page image ("Enable text selection"); written only when
    /// [`PdfOptions::selectable_text`] is on.
    pub text: Option<&'a PdfTextLayer>,
}

/// Invisible, selectable text of one page: glyphs of the text objects in page pixels.
///
/// Each font is written as the standard Type1 base font `Helvetica` (WinAnsi encoding, not
/// embedded) with a `/Widths` array taken from the real font of the layout engine, so selection
/// boxes match the rendered glyphs. The text is drawn in render mode 3 (invisible).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PdfTextLayer {
    pub fonts: Vec<PdfTextFont>,
    pub glyphs: Vec<PdfTextGlyph>,
}

/// A font of a [`PdfTextLayer`].
#[derive(Clone, Debug, PartialEq)]
pub struct PdfTextFont {
    /// Name of the real font (for diagnostics only).
    pub name: String,
    /// Advance widths of the WinAnsi codes 32..=255 in 1/1000 em (224 entries).
    pub widths: Vec<f64>,
}

/// A glyph of a [`PdfTextLayer`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PdfTextGlyph {
    /// Index into [`PdfTextLayer::fonts`].
    pub font: usize,
    pub ch: char,
    /// Text space (1 unit = 1 em, y up) → page pixels (y down): `x = a·u + c·v + e`,
    /// `y = b·u + d·v + f`.
    pub m: [f64; 6],
}

/// WinAnsi codes 0x80..=0x9F that differ from Latin-1.
const WIN_HIGH: [(char, u8); 27] = [
    ('€', 0x80),
    ('‚', 0x82),
    ('ƒ', 0x83),
    ('„', 0x84),
    ('…', 0x85),
    ('†', 0x86),
    ('‡', 0x87),
    ('ˆ', 0x88),
    ('‰', 0x89),
    ('Š', 0x8A),
    ('‹', 0x8B),
    ('Œ', 0x8C),
    ('Ž', 0x8E),
    ('‘', 0x91),
    ('’', 0x92),
    ('“', 0x93),
    ('”', 0x94),
    ('•', 0x95),
    ('–', 0x96),
    ('—', 0x97),
    ('˜', 0x98),
    ('™', 0x99),
    ('š', 0x9A),
    ('›', 0x9B),
    ('œ', 0x9C),
    ('ž', 0x9E),
    ('Ÿ', 0x9F),
];

/// WinAnsi code of a character (`None` = not representable).
pub fn win_ansi(ch: char) -> Option<u8> {
    let c = ch as u32;
    if (0x20..0x7F).contains(&c) || (0xA0..=0xFF).contains(&c) {
        return Some(c as u8);
    }
    WIN_HIGH.iter().find(|(h, _)| *h == ch).map(|(_, b)| *b)
}

/// Character of a WinAnsi code (inverse of [`win_ansi`]; undefined codes give `None`).
pub fn win_ansi_char(code: u8) -> Option<char> {
    match code {
        0x20..=0x7E | 0xA0..=0xFF => Some(code as char),
        _ => WIN_HIGH.iter().find(|(_, b)| *b == code).map(|(c, _)| *c),
    }
}

/// Position corrections below this (1/1000 em) are not written.
const TJ_THRESHOLD: f64 = 1.0;

fn pdf_num(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".into()
    } else {
        s.to_string()
    }
}

fn pdf_string(bytes: &[u8]) -> String {
    let mut s = String::from("(");
    for &b in bytes {
        match b {
            b'(' | b')' | b'\\' => {
                s.push('\\');
                s.push(b as char);
            }
            0x20..=0x7E => s.push(b as char),
            _ => s.push_str(&format!("\\{b:03o}")),
        }
    }
    s.push(')');
    s
}

/// Content-stream operators of the invisible text (`BT 3 Tr … ET`). `w`/`h` = image size in px,
/// `kx`/`ky` = pt per px, `b` = bleed in pt.
fn text_ops(layer: &PdfTextLayer, w: f64, h: f64, kx: f64, ky: f64, b: f64) -> String {
    let _ = w;
    let glyphs: Vec<(&PdfTextGlyph, u8)> = layer
        .glyphs
        .iter()
        .filter(|g| g.font < layer.fonts.len())
        .filter_map(|g| win_ansi(g.ch).map(|c| (g, c)))
        .collect();
    if glyphs.is_empty() {
        return String::new();
    }
    let width = |g: &PdfTextGlyph, c: u8| -> f64 {
        layer.fonts[g.font]
            .widths
            .get((c as usize).wrapping_sub(32))
            .copied()
            .unwrap_or(500.0)
    };
    let mut out = String::from("\nBT 3 Tr");
    let mut cur_font = usize::MAX;
    let mut i = 0;
    while i < glyphs.len() {
        let (g0, _) = glyphs[i];
        let m = g0.m;
        let det = m[0] * m[3] - m[1] * m[2];
        // group: same font and linear part, same baseline, moving forward
        let mut uv = vec![0.0f64];
        let mut j = i + 1;
        while j < glyphs.len() && det.abs() > 1e-12 {
            let g = glyphs[j].0;
            let same = g.font == g0.font
                && (0..4).all(|k| (g.m[k] - m[k]).abs() <= 1e-6 * (1.0 + m[k].abs()));
            if !same {
                break;
            }
            let (dx, dy) = (g.m[4] - m[4], g.m[5] - m[5]);
            let u = (m[3] * dx - m[2] * dy) / det;
            let v = (m[0] * dy - m[1] * dx) / det;
            if v.abs() > 0.01 || u < *uv.last().unwrap_or(&0.0) - 1e-6 {
                break;
            }
            uv.push(u);
            j += 1;
        }
        if g0.font != cur_font {
            cur_font = g0.font;
            out.push_str(&format!("\n/F{cur_font} 1 Tf"));
        }
        let tm = [
            m[0] * kx,
            -m[1] * ky,
            m[2] * kx,
            -m[3] * ky,
            b + m[4] * kx,
            b + (h - m[5]) * ky,
        ];
        out.push('\n');
        for v in tm {
            out.push_str(&pdf_num(v));
            out.push(' ');
        }
        out.push_str("Tm [");
        let mut run: Vec<u8> = Vec::new();
        let mut pen = 0.0f64;
        for k in i..j {
            let (g, c) = glyphs[k];
            run.push(c);
            pen += width(g, c) / 1000.0;
            if k + 1 < j {
                let target = uv[k + 1 - i];
                let adj = (pen - target) * 1000.0;
                // the pen error stays below the threshold because the next step corrects it
                if adj.abs() > TJ_THRESHOLD {
                    out.push_str(&pdf_string(&run));
                    run.clear();
                    out.push(' ');
                    out.push_str(&pdf_num(adj));
                    out.push(' ');
                    pen = target;
                }
            }
        }
        if !run.is_empty() {
            out.push_str(&pdf_string(&run));
        }
        out.push_str("] TJ");
        i = j;
    }
    out.push_str("\nET");
    out
}

/// Producer string of the document info.
pub const PDF_PRODUCER: &str = "Firepixel";

/// Bildkomprimierung im PDF (re\18 §4.1 „Komprimierung:“).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PdfCompression {
    None,
    #[default]
    Zip,
    Jpeg,
    /// „JPEG 2000“: JP2-Bildstrom mit `/JPXDecode` ([`crate::jpx`]; Qualität 100 verlustfrei,
    /// sonst Kompressionsrate aus der Qualität). Gibt es erst ab PDF 1.5: bei niedrigerer
    /// Kompatibilität wird die Datei als PDF 1.5 geschrieben.
    Jpeg2000,
}

/// PDF-Exportoptionen (re\18 §4.1). [`Default`] = bisheriges Verhalten des Befehls `file.export`
/// (PDF 1.4, ZIP, ohne Kennwort).
#[derive(Clone, Debug, PartialEq)]
pub struct PdfOptions {
    /// PDF-Version 3…6 (1.3 … 1.6).
    pub minor_version: u8,
    pub compression: PdfCompression,
    /// JPEG-Qualität 1…100.
    pub jpeg_quality: i32,
    /// Qualitätsstufe der PDF-Optionen (0 = Hoch … 4 = Niedrig): feste Tabellen wie im Original
    /// ([`crate::pdf_jpeg_tables`]); `None` = aus `jpeg_quality` abgeleitet.
    pub quality_level: Option<u8>,
    pub grayscale: bool,
    /// Anschnitt in Punkt rund um die Seite (`/MediaBox` wächst, `/TrimBox` = Seite).
    pub bleed_pt: f64,
    /// Kennwort zum Öffnen (leer = ohne).
    pub open_password: String,
    /// Kennwort zum Einschränken von Aufgaben mit den erlaubten Aufgaben.
    pub permissions_password: Option<(String, crate::pdf_crypt::PdfPermissions)>,
    /// "Enable text selection": write the pages' [`PdfTextLayer`]s as invisible text.
    pub selectable_text: bool,
}

impl Default for PdfOptions {
    fn default() -> Self {
        PdfOptions {
            minor_version: 4,
            compression: PdfCompression::Zip,
            jpeg_quality: 90,
            quality_level: None,
            grayscale: false,
            bleed_pt: 0.0,
            open_password: String::new(),
            permissions_password: None,
            selectable_text: false,
        }
    }
}

/// PDF 1.4 mit je einem Bild pro Seite.
pub fn write_pdf(pages: &[PdfPage], title: &str) -> Result<Vec<u8>> {
    write_pdf_with(pages, title, &PdfOptions::default())
}

/// PDF mit je einem Bild pro Seite nach den Exportoptionen (Version, Komprimierung, Qualität,
/// Graustufen, Anschnitt, Kennwörter).
pub fn write_pdf_with(pages: &[PdfPage], title: &str, opt: &PdfOptions) -> Result<Vec<u8>> {
    if pages.is_empty() {
        return Err(ExportError::Encode(fp_i18n::t("err-export-d-pdf-no-page")));
    }
    let jpx = opt.compression == PdfCompression::Jpeg2000;
    // JPXDecode is defined from PDF 1.5 on: JPEG 2000 raises a lower compatibility setting
    let minor = opt.minor_version.clamp(3, 6).max(if jpx { 5 } else { 3 });
    // Weiche Masken (SMask) gibt es erst ab PDF 1.4: bei 1.3 über Weiß gemischt
    let smask_ok = minor >= 4;
    let protect = !opt.open_password.is_empty()
        || opt
            .permissions_password
            .as_ref()
            .is_some_and(|p| !p.0.is_empty());
    let sec = protect.then(|| {
        let owner = opt
            .permissions_password
            .as_ref()
            .filter(|p| !p.0.is_empty())
            .map(|(pw, perm)| (pw.as_str(), *perm));
        crate::pdf_crypt::PdfSecurity::new(
            &opt.open_password,
            owner,
            minor >= 4,
            format!("{title}|{}|{}", pages.len(), pages[0].image.width()).as_bytes(),
        )
    });
    let stream = |num: usize, dict: &str, data: Vec<u8>| -> Vec<u8> {
        let data = match &sec {
            Some(s) => s.encrypt(num, &data),
            None => data,
        };
        let mut o = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
        o.extend_from_slice(&data);
        o.extend_from_slice(b"\nendstream");
        o
    };
    let flate = opt.compression != PdfCompression::None;
    let mut objs: Vec<Vec<u8>> = Vec::new();
    // 1 Katalog, 2 Seitenbaum, 3 Info, dann je Seite: Seite, Inhalt, Bild, Maske; zuletzt Encrypt
    let n = pages.len();
    let page_obj = |i: usize| 4 + i * 4;
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", page_obj(i))).collect();
    objs.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objs.push(format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).into_bytes());
    let t: String = title
        .chars()
        .filter(|c| c.is_ascii() && *c != '(' && *c != ')' && *c != '\\')
        .collect();
    let info = match &sec {
        Some(s) => {
            let enc = |num: usize, txt: &str| {
                s.encrypt(num, txt.as_bytes())
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect::<String>()
            };
            format!(
                "<< /Title <{}> /Producer <{}> >>",
                enc(3, &t),
                enc(3, PDF_PRODUCER)
            )
        }
        None => format!("<< /Title ({t}) /Producer ({PDF_PRODUCER}) >>"),
    };
    objs.push(info.into_bytes());
    let b = opt.bleed_pt.max(0.0);
    // font objects of the invisible text follow the pages (one per page font)
    let mut next_font_obj = 4 + n * 4;
    let mut font_objs: Vec<Vec<u8>> = Vec::new();
    for (i, p) in pages.iter().enumerate() {
        let (w, h) = (p.image.width(), p.image.height());
        if w == 0 || h == 0 {
            return Err(ExportError::BadSize(w, h));
        }
        let base = page_obj(i);
        let mut content = format!(
            "q {:.4} 0 0 {:.4} {b:.4} {b:.4} cm /Im{i} Do Q",
            p.width_pt, p.height_pt
        );
        let mut font_res = String::new();
        if let Some(tl) = p.text.filter(|_| opt.selectable_text) {
            let ops = text_ops(
                tl,
                w as f64,
                h as f64,
                p.width_pt / w as f64,
                p.height_pt / h as f64,
                b,
            );
            if !ops.is_empty() {
                content.push_str(&ops);
                font_res.push_str(" /Font <<");
                for (k, f) in tl.fonts.iter().enumerate() {
                    let widths: Vec<String> = (0..224)
                        .map(|c| pdf_num(f.widths.get(c).copied().unwrap_or(500.0)))
                        .collect();
                    font_res.push_str(&format!(" /F{k} {next_font_obj} 0 R"));
                    font_objs.push(format!(
                        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding /FirstChar 32 /LastChar 255 /Widths [{}] >>",
                        widths.join(" ")
                    ).into_bytes());
                    next_font_obj += 1;
                }
                font_res.push_str(" >>");
            }
        }
        let (mw, mh) = (p.width_pt + 2.0 * b, p.height_pt + 2.0 * b);
        let boxes = if b > 0.0 {
            format!(
                " /BleedBox [0 0 {mw:.4} {mh:.4}] /TrimBox [{b:.4} {b:.4} {:.4} {:.4}]",
                b + p.width_pt,
                b + p.height_pt
            )
        } else {
            String::new()
        };
        objs.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {mw:.4} {mh:.4}]{boxes} /Resources << /XObject << /Im{i} {} 0 R >>{font_res} >> /Contents {} 0 R >>", base + 2, base + 1).into_bytes());
        objs.push(if flate {
            stream(
                base + 1,
                "/Filter /FlateDecode",
                deflate(content.as_bytes())?,
            )
        } else {
            stream(base + 1, "", content.into_bytes())
        });
        let alpha: Vec<u8> = p.image.data().iter().map(|px| px[3]).collect();
        let has_alpha = alpha.iter().any(|&a| a != 255);
        let use_smask = has_alpha && smask_ok;
        // Farbwerte: ohne SMask über Weiß gemischt; Graustufen nach Rec. 601
        let rgb_of = |px: &[u8; 4]| -> [u8; 3] {
            let mix = |c: u8| {
                if use_smask {
                    c
                } else {
                    ((c as u32 * px[3] as u32 + 255 * (255 - px[3] as u32)) / 255) as u8
                }
            };
            let (r, g, bb) = (mix(px[0]), mix(px[1]), mix(px[2]));
            if opt.grayscale {
                let y = ((r as u32 * 299 + g as u32 * 587 + bb as u32 * 114) / 1000) as u8;
                [y, y, y]
            } else {
                [r, g, bb]
            }
        };
        let smask = if use_smask {
            format!(" /SMask {} 0 R", base + 3)
        } else {
            String::new()
        };
        let cs = if opt.grayscale {
            "/DeviceGray"
        } else {
            "/DeviceRGB"
        };
        let img = match opt.compression {
            PdfCompression::Jpeg2000 => {
                // like the JPEG path: alpha goes into the separate /SMask (or is mixed over white
                // below PDF 1.4), the JP2 stream carries only the colour (one component in
                // grayscale). /ColorSpace is written although the JP2 `colr` box says the same,
                // so readers that ignore the box agree.
                let comps = if opt.grayscale { 1 } else { 3 };
                let samples: Vec<u8> = p
                    .image
                    .data()
                    .iter()
                    .flat_map(|q| {
                        let c = rgb_of(q);
                        c.into_iter().take(comps)
                    })
                    .collect();
                let jp2 =
                    crate::jpx::encode_jp2(&samples, w as u32, h as u32, comps, opt.jpeg_quality)?;
                stream(
                    base + 2,
                    &format!(
                        "/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace {cs} /BitsPerComponent 8 /Filter /JPXDecode{smask}"
                    ),
                    jp2,
                )
            }
            PdfCompression::Jpeg => {
                let px: Vec<u32> = p
                    .image
                    .data()
                    .iter()
                    .map(|q| {
                        let c = rgb_of(q);
                        crate::px::argb(255, c[0] as u32, c[1] as u32, c[2] as u32)
                    })
                    .collect();
                let fixed = opt.quality_level.map(|l| {
                    Box::new(crate::pdf_jpeg_tables::PDF_JPEG_TABLES[(l as usize).min(4)])
                });
                let params = crate::jpeg::JpegParams {
                    quality: opt.jpeg_quality.clamp(1, 100),
                    optimize: true,
                    fixed_tables: fixed,
                    ..crate::jpeg::JpegParams::default()
                };
                let jpg = crate::jpeg::encode_jpeg(
                    &px,
                    w,
                    h,
                    &params,
                    &crate::jpeg_tables::FwJpegTables::ijg_fallback(),
                )?;
                stream(
                    base + 2,
                    &format!(
                        "/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode{smask}"
                    ),
                    jpg,
                )
            }
            c => {
                let data: Vec<u8> = if opt.grayscale {
                    p.image.data().iter().map(|q| rgb_of(q)[0]).collect()
                } else {
                    p.image.data().iter().flat_map(rgb_of).collect()
                };
                let (filter, data) = if c == PdfCompression::None {
                    ("", data)
                } else {
                    (" /Filter /FlateDecode", deflate(&data)?)
                };
                stream(
                    base + 2,
                    &format!(
                        "/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace {cs} /BitsPerComponent 8{filter}{smask}"
                    ),
                    data,
                )
            }
        };
        objs.push(img);
        let adata = if use_smask { alpha } else { Vec::new() };
        let (filter, adata) = if flate {
            (" /Filter /FlateDecode", deflate(&adata)?)
        } else {
            ("", adata)
        };
        objs.push(stream(base + 3, &format!("/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceGray /BitsPerComponent 8{filter}"), adata));
    }
    objs.extend(font_objs);
    if let Some(s) = &sec {
        objs.push(s.dict().into_bytes());
    }
    let mut out = format!("%PDF-1.{minor}\n").into_bytes();
    out.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n");
    let mut offsets = Vec::with_capacity(objs.len());
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for off in offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    let extra = match &sec {
        Some(s) => format!(
            " /Encrypt {} 0 R /ID [<{id}> <{id}>]",
            objs.len(),
            id = s.id_hex()
        ),
        None => String::new(),
    };
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /Info 3 0 R{extra} >>\nstartxref\n{xref}\n%%EOF\n",
            objs.len() + 1
        )
        .as_bytes(),
    );
    Ok(out)
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64 (RFC 4648, mit Auffüllung).
pub fn base64(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for ch in data.chunks(3) {
        let v = (ch[0] as u32) << 16
            | (ch.get(1).copied().unwrap_or(0) as u32) << 8
            | ch.get(2).copied().unwrap_or(0) as u32;
        for k in 0..4 {
            if k <= ch.len() {
                s.push(B64[((v >> (18 - 6 * k)) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

fn png_data_uri(img: &RgbaImage) -> Option<String> {
    let px: Vec<u32> = img.data().iter().map(|&p| from_rgba(p)).collect();
    let png = write_png_truecolor(
        &px,
        img.width(),
        img.height(),
        true,
        &PngOptions {
            dpi: None,
            ..PngOptions::default()
        },
    )
    .ok()?;
    Some(format!("data:image/png;base64,{}", base64(&png)))
}

fn color(c: fp_color::Argb) -> (String, f32) {
    (
        format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b()),
        c.a() as f32 / 255.0,
    )
}

fn path_d(p: &fp_model::PathElem, ox: f32, oy: f32) -> String {
    let mut d = String::new();
    for c in &p.contours {
        let Some(first) = c.nodes.first() else {
            continue;
        };
        d.push_str(&format!(
            "M{} {} ",
            first.anchor.x - ox,
            first.anchor.y - oy
        ));
        let n = c.nodes.len();
        let segs = if c.closed { n } else { n.saturating_sub(1) };
        for i in 0..segs {
            let a = &c.nodes[i];
            let b = &c.nodes[(i + 1) % n];
            d.push_str(&format!(
                "C{} {} {} {} {} {} ",
                a.succ.x - ox,
                a.succ.y - oy,
                b.pred.x - ox,
                b.pred.y - oy,
                b.anchor.x - ox,
                b.anchor.y - oy
            ));
        }
        if c.closed {
            d.push_str("Z ");
        }
    }
    d.trim_end().to_string()
}

fn element_svg(out: &mut String, e: &Element, ox: f32, oy: f32, depth: u32) {
    if !e.common.visible || depth > 64 {
        return;
    }
    let op = e.common.opacity.0 as f32 / 255.0;
    match &e.kind {
        ElementKind::Path(p) => {
            let a = &p.attrs;
            let (fill, fo) = if a.fill.is_some() {
                color(a.fill_color)
            } else {
                ("none".into(), 1.0)
            };
            let (stroke, so) = if a.brush.is_some() {
                color(a.stroke_color)
            } else {
                ("none".into(), 1.0)
            };
            let sw = a
                .brush
                .as_ref()
                .map(|b| b.diameter.max(1) as f32)
                .unwrap_or(0.0);
            let rule = if p.even_odd { "evenodd" } else { "nonzero" };
            out.push_str(&format!(
                "<path d=\"{}\" fill=\"{fill}\" fill-opacity=\"{fo}\" fill-rule=\"{rule}\" stroke=\"{stroke}\" stroke-opacity=\"{so}\" stroke-width=\"{sw}\" opacity=\"{op}\"/>\n",
                path_d(p, ox, oy)
            ));
        }
        ElementKind::Bitmap(b) => {
            let img = b
                .effect_result
                .as_ref()
                .map(|r| &r.bitmap.image)
                .unwrap_or(&b.image);
            let (rgba, _) = img.to_rgba();
            let pos = img.position();
            let r = fp_geom::IRect::new(0, 0, img.grid.width as i32, img.grid.height as i32);
            if let Ok(i) = RgbaImage::from_rgba_bytes(r, &rgba)
                && let Some(uri) = png_data_uri(&i)
            {
                out.push_str(&format!(
                    "<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" opacity=\"{op}\" href=\"{uri}\"/>\n",
                    pos.x - ox,
                    pos.y - oy,
                    img.grid.width,
                    img.grid.height
                ));
            }
        }
        ElementKind::Group(g) => {
            out.push_str(&format!("<g opacity=\"{op}\">\n"));
            for c in g.children.iter().rev() {
                element_svg(out, c, ox, oy, depth + 1);
            }
            out.push_str("</g>\n");
        }
        ElementKind::SubLayer(l) => {
            if let Some(cel) = l.cels.first() {
                for c in cel.elements.iter().rev() {
                    element_svg(out, c, ox, oy, depth + 1);
                }
            }
        }
        _ => {
            if let Some(c) = &e.cache {
                let img = &c.bitmap.image;
                let (rgba, _) = img.to_rgba();
                let pos = img.position();
                let r = fp_geom::IRect::new(0, 0, img.grid.width as i32, img.grid.height as i32);
                if let Ok(i) = RgbaImage::from_rgba_bytes(r, &rgba)
                    && let Some(uri) = png_data_uri(&i)
                {
                    out.push_str(&format!(
                        "<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" opacity=\"{op}\" href=\"{uri}\"/>\n",
                        pos.x - ox,
                        pos.y - oy,
                        img.grid.width,
                        img.grid.height
                    ));
                }
            }
        }
    }
}

/// SVG einer Seite. `flat` = optional flach gerechnetes Seitenbild (dann nur dieses, als Bild).
pub fn write_svg(
    doc: &Document,
    page: usize,
    frame: usize,
    flat: Option<&RgbaImage>,
) -> Result<String> {
    let p = doc.pages.get(page).ok_or_else(|| {
        ExportError::Encode(fp_i18n::tr!(
            "err-export-d-page-missing",
            page = page.to_string()
        ))
    })?;
    let (w, h) = (p.canvas.width, p.canvas.height);
    let (ox, oy) = (p.canvas.origin.x, p.canvas.origin.y);
    let mut out = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n"
    );
    let (bg, bo) = color(p.root.background);
    if bo > 0.0 {
        out.push_str(&format!(
            "<rect width=\"{w}\" height=\"{h}\" fill=\"{bg}\" fill-opacity=\"{bo}\"/>\n"
        ));
    }
    if let Some(img) = flat {
        if let Some(uri) = png_data_uri(img) {
            out.push_str(&format!(
                "<image x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" href=\"{uri}\"/>\n",
                img.width(),
                img.height()
            ));
        }
    } else {
        for slot in &p.root.layers {
            let LayerSlot::Own(l) = slot else { continue };
            if l.kind == LayerKind::Web || !l.is_visible() {
                continue;
            }
            out.push_str(&format!("<g id=\"{}\">\n", l.name.replace('"', "'")));
            if let Some(cel) = l.cel(frame) {
                for e in cel.elements.iter().rev() {
                    element_svg(&mut out, e, ox, oy, 0);
                }
            }
            out.push_str("</g>\n");
        }
    }
    out.push_str("</svg>\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_model::{PathAttrs, Rect};
    use std::sync::Arc;

    #[test]
    fn pdf_and_svg() {
        let img = RgbaImage::filled(
            fp_geom::IRect::new(0, 0, 3, 2),
            fp_color::Rgba8::new(1, 2, 3, 128),
        )
        .unwrap();
        let pdf = write_pdf(
            &[PdfPage {
                image: &img,
                width_pt: 3.0,
                height_pt: 2.0,
                text: None,
            }],
            "t",
        )
        .unwrap();
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        let mut d = Document::new(10.0, 10.0, fp_color::Argb(0xFFFFFFFF));
        let mut ids = std::mem::take(&mut d.ids);
        let r = Element::new_rectangle(
            &mut ids,
            Rect::new(1.0, 1.0, 5.0, 5.0),
            0.0,
            PathAttrs::solid_fill(fp_color::Argb(0xFFFF0000)),
        );
        d.ids = ids;
        if let Some(LayerSlot::Own(l)) = d.page_mut(0).map(|p| &mut p.root.layers[0]) {
            Arc::make_mut(&mut Arc::make_mut(l).cels[0])
                .elements
                .push(Arc::new(r));
        }
        let svg = write_svg(&d, 0, 0, None).unwrap();
        assert!(svg.contains("<path d=\"M1 1 C1 1 5 1 5 1"));
        assert!(svg.contains("fill=\"#ff0000\""));
    }

    /// Content stream of the first page (object 5), uncompressed.
    fn page_content(pdf: &[u8]) -> String {
        let s = String::from_utf8_lossy(pdf).to_string();
        let o = s.find("5 0 obj").expect("content object");
        let a = s[o..]
            .find(
                "stream
",
            )
            .expect("stream")
            + o
            + 7;
        let b = s[a..]
            .find(
                "
endstream",
            )
            .expect("endstream")
            + a;
        s[a..b].to_string()
    }

    fn text_page_pdf(selectable: bool, compression: PdfCompression) -> Vec<u8> {
        let img = RgbaImage::filled(
            fp_geom::IRect::new(0, 0, 100, 50),
            fp_color::Rgba8::new(255, 255, 255, 255),
        )
        .unwrap();
        let mut widths = vec![600.0; 224];
        widths[(b'i' - 32) as usize] = 250.0;
        let em = |x: f64| [20.0, 0.0, 0.0, -20.0, x, 30.0];
        let layer = PdfTextLayer {
            fonts: vec![PdfTextFont {
                name: "Test".into(),
                widths,
            }],
            glyphs: vec![
                PdfTextGlyph {
                    font: 0,
                    ch: 'H',
                    m: em(10.0),
                },
                // H advances 600/1000 em = 12 px: no adjustment
                PdfTextGlyph {
                    font: 0,
                    ch: 'i',
                    m: em(22.0),
                },
                // i advances 5 px, 3 px tracking: -150/1000 em
                PdfTextGlyph {
                    font: 0,
                    ch: '(',
                    m: em(30.0),
                },
            ],
        };
        write_pdf_with(
            &[PdfPage {
                image: &img,
                width_pt: 100.0,
                height_pt: 50.0,
                text: Some(&layer),
            }],
            "t",
            &PdfOptions {
                compression,
                selectable_text: selectable,
                ..PdfOptions::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn pdf_selectable_text_is_invisible_text_over_the_image() {
        let pdf = text_page_pdf(true, PdfCompression::None);
        let c = page_content(&pdf);
        let toks: Vec<&str> = c.split_whitespace().collect();
        // image first, then the invisible text
        assert!(c.starts_with("q 100.0000 0 0 50.0000"), "{c}");
        let bt = toks.iter().position(|t| *t == "BT").expect("BT");
        assert_eq!(&toks[bt..bt + 3], ["BT", "3", "Tr"]);
        assert_eq!(&toks[bt + 3..bt + 6], ["/F0", "1", "Tf"]);
        // Tm: 20 px em at x 10, baseline y 30 px from the top of a 50 pt page
        assert_eq!(
            &toks[bt + 6..bt + 13],
            ["20", "0", "0", "20", "10", "20", "Tm"]
        );
        assert_eq!(&toks[bt + 13..], ["[(Hi)", "-150", r"(\()]", "TJ", "ET"]);
        let s = String::from_utf8_lossy(&pdf);
        assert!(s.contains("/Font << /F0 "), "font resource");
        assert!(s.contains("/BaseFont /Helvetica /Encoding /WinAnsiEncoding /FirstChar 32"));
        assert!(s.contains("/Widths [600 600"));
        // switched off: no text
        let off = text_page_pdf(false, PdfCompression::None);
        assert!(!page_content(&off).contains("Tr"));
        assert!(!String::from_utf8_lossy(&off).contains("/Font"));
    }

    /// Pattern image `w × h` with alpha `a` (every pixel different in each channel).
    fn pattern_image(w: i32, h: i32, a: u8) -> RgbaImage {
        let mut bytes = Vec::new();
        for y in 0..h {
            for x in 0..w {
                bytes.extend_from_slice(&[
                    (x * 37 + y * 5) as u8,
                    (y * 29 + x * 3) as u8,
                    ((x ^ y) * 7) as u8,
                    a,
                ]);
            }
        }
        RgbaImage::from_rgba_bytes(fp_geom::IRect::new(0, 0, w, h), &bytes).unwrap()
    }

    fn pdf_of(img: &RgbaImage, opt: &PdfOptions) -> Vec<u8> {
        write_pdf_with(
            &[PdfPage {
                image: img,
                width_pt: img.width() as f64,
                height_pt: img.height() as f64,
                text: None,
            }],
            "t",
            opt,
        )
        .unwrap()
    }

    /// Dictionary and raw data of the first stream whose dictionary contains `key` (byte search:
    /// earlier streams are binary).
    fn stream_with<'a>(pdf: &'a [u8], key: &str) -> (String, &'a [u8]) {
        let find = |needle: &[u8], from: usize| {
            pdf[from..]
                .windows(needle.len())
                .position(|w| w == needle)
                .map(|i| i + from)
        };
        let k = find(key.as_bytes(), 0).unwrap_or_else(|| panic!("{key} missing"));
        let start = (0..k).rev().find(|&i| pdf[i..].starts_with(b"<<")).unwrap();
        const HEAD: &[u8] = b">>\nstream\n";
        let st = find(HEAD, k).unwrap();
        let dict = String::from_utf8_lossy(&pdf[start..st + 2]).to_string();
        let len: usize = dict
            .split("/Length ")
            .nth(1)
            .and_then(|r| r.split_whitespace().next())
            .unwrap()
            .parse()
            .unwrap();
        let data = st + HEAD.len();
        (dict, &pdf[data..data + len])
    }

    #[test]
    fn pdf_jpeg2000_lossless_is_jpxdecode_and_pixel_equal() {
        let img = pattern_image(37, 21, 255);
        let pdf = pdf_of(
            &img,
            &PdfOptions {
                compression: PdfCompression::Jpeg2000,
                jpeg_quality: 100,
                ..PdfOptions::default()
            },
        );
        // JPXDecode needs PDF 1.5: the default 1.4 is raised
        assert!(pdf.starts_with(b"%PDF-1.5"));
        let s = String::from_utf8_lossy(&pdf);
        assert!(!s.contains("DCTDecode"));
        assert!(s.contains("/Producer (Firepixel)"));
        let (dict, jp2) = stream_with(&pdf, "/Filter /JPXDecode");
        assert!(
            dict.contains("/Width 37 /Height 21 /ColorSpace /DeviceRGB"),
            "{dict}"
        );
        assert!(!dict.contains("/SMask"), "opaque: no mask");
        assert!(jp2.starts_with(openjp2::JP2_RFC3745_MAGIC), "JP2 file");
        let src: Vec<u8> = img.data().iter().flat_map(|q| [q[0], q[1], q[2]]).collect();
        // decoded with the JPEG 2000 decoder of the PDF import
        let (w, h, n, px) = crate::jpx::tests::decode(jp2);
        assert_eq!((w, h, n), (37, 21, 3));
        assert_eq!(px, src, "lossless = pixel-equal");
    }

    #[test]
    fn pdf_jpeg2000_lossy_alpha_and_grayscale() {
        // half-transparent: alpha goes into the /SMask as in the JPEG path
        let img = pattern_image(40, 30, 128);
        let opt = PdfOptions {
            compression: PdfCompression::Jpeg2000,
            minor_version: 6,
            jpeg_quality: 60,
            ..PdfOptions::default()
        };
        let pdf = pdf_of(&img, &opt);
        assert!(pdf.starts_with(b"%PDF-1.6"), "higher versions stay");
        let (dict, jp2) = stream_with(&pdf, "/Filter /JPXDecode");
        assert!(dict.contains("/SMask "), "{dict}");
        let (w, h, n, _) = crate::jpx::tests::decode(jp2);
        assert_eq!((w, h, n), (40, 30, 3));
        let lossless = pdf_of(
            &img,
            &PdfOptions {
                jpeg_quality: 100,
                ..opt.clone()
            },
        );
        assert!(stream_with(&lossless, "/Filter /JPXDecode").1.len() > jp2.len());
        // grayscale: one component, /DeviceGray
        let gray = pdf_of(
            &img,
            &PdfOptions {
                grayscale: true,
                jpeg_quality: 100,
                ..opt.clone()
            },
        );
        let (dict, jp2) = stream_with(&gray, "/Filter /JPXDecode");
        assert!(dict.contains("/ColorSpace /DeviceGray"), "{dict}");
        let (_, _, n, px) = crate::jpx::tests::decode(jp2);
        assert_eq!(n, 1);
        let want: Vec<u8> = img
            .data()
            .iter()
            .map(|q| ((q[0] as u32 * 299 + q[1] as u32 * 587 + q[2] as u32 * 114) / 1000) as u8)
            .collect();
        assert_eq!(px, want);
        // PDF 1.3 (no SMask): raised to 1.5, which allows the mask again
        let old = pdf_of(
            &img,
            &PdfOptions {
                minor_version: 3,
                ..opt
            },
        );
        assert!(old.starts_with(b"%PDF-1.5"));
        // other compressions keep their version and the plain producer
        let zip = text_page_pdf(false, PdfCompression::Zip);
        assert!(zip.starts_with(b"%PDF-1.4"));
        assert!(String::from_utf8_lossy(&zip).contains("/Producer (Firepixel)"));
    }
}
