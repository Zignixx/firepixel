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
}

/// Bildkomprimierung im PDF (re\18 §4.1 „Komprimierung:“).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PdfCompression {
    None,
    #[default]
    Zip,
    Jpeg,
    /// „JPEG 2000“: Firepixel hat keinen JPEG-2000-Kodierer und schreibt JPEG (DCT) in der
    /// gewählten Qualität.
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
    let minor = opt.minor_version.clamp(3, 6);
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
                enc(3, "Firepixel")
            )
        }
        None => format!("<< /Title ({t}) /Producer (Firepixel) >>"),
    };
    objs.push(info.into_bytes());
    let b = opt.bleed_pt.max(0.0);
    for (i, p) in pages.iter().enumerate() {
        let (w, h) = (p.image.width(), p.image.height());
        if w == 0 || h == 0 {
            return Err(ExportError::BadSize(w, h));
        }
        let base = page_obj(i);
        let content = format!(
            "q {:.4} 0 0 {:.4} {b:.4} {b:.4} cm /Im{i} Do Q",
            p.width_pt, p.height_pt
        );
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
        objs.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {mw:.4} {mh:.4}]{boxes} /Resources << /XObject << /Im{i} {} 0 R >> >> /Contents {} 0 R >>", base + 2, base + 1).into_bytes());
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
            PdfCompression::Jpeg | PdfCompression::Jpeg2000 => {
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
                    p.image.data().iter().flat_map(|q| rgb_of(q)).collect()
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
}
