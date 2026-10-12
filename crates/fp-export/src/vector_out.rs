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

use crate::png_out::{write_png_truecolor, PngOptions};
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

/// PDF 1.4 mit je einem Bild pro Seite.
pub fn write_pdf(pages: &[PdfPage], title: &str) -> Result<Vec<u8>> {
    if pages.is_empty() {
        return Err(ExportError::Encode(fp_i18n::t("err-export-d-pdf-no-page")));
    }
    let mut objs: Vec<Vec<u8>> = Vec::new();
    // 1 Katalog, 2 Seitenbaum, 3 Info, dann je Seite: Seite, Inhalt, Bild, Maske
    let n = pages.len();
    let page_obj = |i: usize| 4 + i * 4;
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", page_obj(i))).collect();
    objs.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objs.push(format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).into_bytes());
    let t: String = title.chars().filter(|c| c.is_ascii() && *c != '(' && *c != ')' && *c != '\\').collect();
    objs.push(format!("<< /Title ({t}) /Producer (Firepixel) >>").into_bytes());
    for (i, p) in pages.iter().enumerate() {
        let (w, h) = (p.image.width(), p.image.height());
        if w == 0 || h == 0 {
            return Err(ExportError::BadSize(w, h));
        }
        let base = page_obj(i);
        let content = format!("q {:.4} 0 0 {:.4} 0 0 cm /Im{i} Do Q", p.width_pt, p.height_pt);
        objs.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.4} {:.4}] /Resources << /XObject << /Im{i} {} 0 R >> >> /Contents {} 0 R >>",
                p.width_pt,
                p.height_pt,
                base + 2,
                base + 1
            )
            .into_bytes(),
        );
        let cdata = deflate(content.as_bytes())?;
        let mut o = format!("<< /Length {} /Filter /FlateDecode >>\nstream\n", cdata.len()).into_bytes();
        o.extend_from_slice(&cdata);
        o.extend_from_slice(b"\nendstream");
        objs.push(o);
        let rgb: Vec<u8> = p.image.data().iter().flat_map(|px| [px[0], px[1], px[2]]).collect();
        let alpha: Vec<u8> = p.image.data().iter().map(|px| px[3]).collect();
        let has_alpha = alpha.iter().any(|&a| a != 255);
        let idata = deflate(&rgb)?;
        let smask = if has_alpha { format!(" /SMask {} 0 R", base + 3) } else { String::new() };
        let mut o = format!("<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode{smask} /Length {} >>\nstream\n", idata.len()).into_bytes();
        o.extend_from_slice(&idata);
        o.extend_from_slice(b"\nendstream");
        objs.push(o);
        let adata = deflate(if has_alpha { &alpha } else { &[] })?;
        let mut o = format!("<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode /Length {} >>\nstream\n", adata.len()).into_bytes();
        o.extend_from_slice(&adata);
        o.extend_from_slice(b"\nendstream");
        objs.push(o);
    }
    let mut out = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
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
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R /Info 3 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).as_bytes());
    Ok(out)
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64 (RFC 4648, mit Auffüllung).
pub fn base64(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for ch in data.chunks(3) {
        let v = (ch[0] as u32) << 16 | (ch.get(1).copied().unwrap_or(0) as u32) << 8 | ch.get(2).copied().unwrap_or(0) as u32;
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
    let png = write_png_truecolor(&px, img.width(), img.height(), true, &PngOptions { dpi: None, ..PngOptions::default() }).ok()?;
    Some(format!("data:image/png;base64,{}", base64(&png)))
}

fn color(c: fp_color::Argb) -> (String, f32) {
    (format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b()), c.a() as f32 / 255.0)
}

fn path_d(p: &fp_model::PathElem, ox: f32, oy: f32) -> String {
    let mut d = String::new();
    for c in &p.contours {
        let Some(first) = c.nodes.first() else { continue };
        d.push_str(&format!("M{} {} ", first.anchor.x - ox, first.anchor.y - oy));
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
            let (fill, fo) = if a.fill.is_some() { color(a.fill_color) } else { ("none".into(), 1.0) };
            let (stroke, so) = if a.brush.is_some() { color(a.stroke_color) } else { ("none".into(), 1.0) };
            let sw = a.brush.as_ref().map(|b| b.diameter.max(1) as f32).unwrap_or(0.0);
            let rule = if p.even_odd { "evenodd" } else { "nonzero" };
            out.push_str(&format!(
                "<path d=\"{}\" fill=\"{fill}\" fill-opacity=\"{fo}\" fill-rule=\"{rule}\" stroke=\"{stroke}\" stroke-opacity=\"{so}\" stroke-width=\"{sw}\" opacity=\"{op}\"/>\n",
                path_d(p, ox, oy)
            ));
        }
        ElementKind::Bitmap(b) => {
            let img = b.effect_result.as_ref().map(|r| &r.bitmap.image).unwrap_or(&b.image);
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
pub fn write_svg(doc: &Document, page: usize, frame: usize, flat: Option<&RgbaImage>) -> Result<String> {
    let p = doc.pages.get(page).ok_or_else(|| ExportError::Encode(fp_i18n::tr!("err-export-d-page-missing", page = page.to_string())))?;
    let (w, h) = (p.canvas.width, p.canvas.height);
    let (ox, oy) = (p.canvas.origin.x, p.canvas.origin.y);
    let mut out = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n");
    let (bg, bo) = color(p.root.background);
    if bo > 0.0 {
        out.push_str(&format!("<rect width=\"{w}\" height=\"{h}\" fill=\"{bg}\" fill-opacity=\"{bo}\"/>\n"));
    }
    if let Some(img) = flat {
        if let Some(uri) = png_data_uri(img) {
            out.push_str(&format!("<image x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" href=\"{uri}\"/>\n", img.width(), img.height()));
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
        let img = RgbaImage::filled(fp_geom::IRect::new(0, 0, 3, 2), fp_color::Rgba8::new(1, 2, 3, 128)).unwrap();
        let pdf = write_pdf(&[PdfPage { image: &img, width_pt: 3.0, height_pt: 2.0 }], "t").unwrap();
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        let mut d = Document::new(10.0, 10.0, fp_color::Argb(0xFFFFFFFF));
        let mut ids = std::mem::take(&mut d.ids);
        let r = Element::new_rectangle(&mut ids, Rect::new(1.0, 1.0, 5.0, 5.0), 0.0, PathAttrs::solid_fill(fp_color::Argb(0xFFFF0000)));
        d.ids = ids;
        if let Some(LayerSlot::Own(l)) = d.page_mut(0).map(|p| &mut p.root.layers[0]) {
            Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements.push(Arc::new(r));
        }
        let svg = write_svg(&d, 0, 0, None).unwrap();
        assert!(svg.contains("<path d=\"M1 1 C1 1 5 1 5 1"));
        assert!(svg.contains("fill=\"#ff0000\""));
    }
}
