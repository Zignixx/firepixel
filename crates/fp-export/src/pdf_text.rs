//! Text objects of a page as invisible PDF text ("Enable text selection", BA-100).
//!
//! The page itself stays the flattened image; [`text_layer`] lays out every visible text object
//! with the text engine (same layout as the renderer) and returns its glyphs with their exact
//! text-space matrices in page pixels, plus one width table per real font (advance widths of the
//! WinAnsi characters in 1/1000 em). [`crate::vector_out`] writes them in render mode 3.
//!
//! Not covered: text inside symbol instances, characters outside WinAnsi (they are skipped).

use std::collections::HashMap;

use fp_model::{Document, Element, ElementKind, ElementState, LayerKind, LayerSlot};
use fp_text::FontDb;

use crate::vector_out::{PdfTextFont, PdfTextGlyph, PdfTextLayer, win_ansi_char};

/// Invisible text layer of page `page` (frame 0). Coordinates relative to the page canvas
/// (`page_rect`), in pixels.
pub fn text_layer(doc: &Document, page: usize, fonts: &FontDb) -> PdfTextLayer {
    let mut out = PdfTextLayer::default();
    let Some(p) = doc.pages.get(page) else {
        return out;
    };
    let origin = crate::render::page_rect(doc, page)
        .map(|r| (r.left as f64, r.top as f64))
        .unwrap_or((0.0, 0.0));
    let mut font_ix: HashMap<usize, usize> = HashMap::new();
    let mut texts: Vec<&fp_model::TextElem> = Vec::new();
    for slot in &p.root.layers {
        let LayerSlot::Own(l) = slot else { continue };
        if l.kind == LayerKind::Web || !l.is_shown(0) {
            continue;
        }
        if let Some(cel) = l.cel(0) {
            // bottom to top, like the renderer
            for e in cel.elements.iter().rev() {
                collect(e, &mut texts, 0);
            }
        }
    }
    for t in texts {
        let block = fp_render::convert::text_block(doc, t);
        let lay = fp_text::layout(&block, fonts, &fp_text::LayoutOptions::default());
        let obj = t.transform.as_ref().map(|tm| fp_text::raster::Affine {
            a: tm.m[0][0] as f64,
            b: tm.m[0][1] as f64,
            c: tm.m[1][0] as f64,
            d: tm.m[1][1] as f64,
            e: tm.m[2][0] as f64,
            f: tm.m[2][1] as f64,
        });
        for g in lay.visible_glyphs() {
            if g.is_tab || g.ch.is_control() {
                continue;
            }
            let upem = g.font.units_per_em().max(1) as f64;
            let mut m = g.transform;
            if let Some(o) = &obj {
                m = m.then(o);
            }
            let key = std::sync::Arc::as_ptr(&g.font) as usize;
            let fi = *font_ix.entry(key).or_insert_with(|| {
                let f = &g.font;
                let widths = (32u8..=255)
                    .map(|code| {
                        win_ansi_char(code)
                            .map(|c| f.advance(f.glyph_index(c)) as f64 * 1000.0 / upem)
                            .unwrap_or(500.0)
                    })
                    .collect();
                out.fonts.push(PdfTextFont {
                    name: f.ps_name().to_string(),
                    widths,
                });
                out.fonts.len() - 1
            });
            out.glyphs.push(PdfTextGlyph {
                font: fi,
                ch: g.ch,
                m: [
                    m.a * upem,
                    m.b * upem,
                    m.c * upem,
                    m.d * upem,
                    m.e - origin.0,
                    m.f - origin.1,
                ],
            });
        }
    }
    out
}

fn collect<'a>(e: &'a Element, out: &mut Vec<&'a fp_model::TextElem>, depth: u32) {
    if !e.common.visible || depth > 64 {
        return;
    }
    match &e.kind {
        ElementKind::Text(t) => out.push(t),
        ElementKind::Group(g) => {
            for c in g.children.iter().rev() {
                collect(c, out, depth + 1);
            }
        }
        ElementKind::SubLayer(l) => {
            if l.is_visible()
                && let Some(cel) = l.cel_at(0, 1, ElementState::Normal)
            {
                for c in cel.elements.iter().rev() {
                    collect(c, out, depth + 1);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn text_object_becomes_positioned_glyphs() {
        let fonts = FontDb::shared();
        if fonts.is_empty() {
            return; // no system fonts on this machine
        }
        let mut d = Document::new(200.0, 100.0, fp_color::Argb(0xFFFFFFFF));
        let mut ids = std::mem::take(&mut d.ids);
        let t = Element::new_text(
            &mut ids,
            "Hello",
            "ArialMT",
            20.0,
            fp_color::Argb(0xFF000000),
            fp_geom::Point::new(10.0, 40.0),
        );
        d.ids = ids;
        if let Some(LayerSlot::Own(l)) = d.page_mut(0).map(|p| &mut p.root.layers[0]) {
            Arc::make_mut(&mut Arc::make_mut(l).cels[0])
                .elements
                .push(Arc::new(t));
        }
        let lay = text_layer(&d, 0, &fonts);
        let s: String = lay.glyphs.iter().map(|g| g.ch).collect();
        assert_eq!(s, "Hello");
        assert_eq!(lay.fonts.len(), 1);
        let g = lay.glyphs[0];
        // 20 px em, y up in text space
        assert!(
            (g.m[0] - 20.0).abs() < 0.5 && (g.m[3] + 20.0).abs() < 0.5,
            "{:?}",
            g.m
        );
        assert!(g.m[4] >= 9.0 && g.m[4] <= 13.0, "{:?}", g.m);
        // glyphs advance by the font's widths
        let w_h = lay.fonts[0].widths[(b'H' - 32) as usize];
        let dx = lay.glyphs[1].m[4] - g.m[4];
        assert!((dx - w_h * 20.0 / 1000.0).abs() < 1.0, "{dx} vs {w_h}");
        let img = fp_raster::RgbaImage::filled(
            fp_geom::IRect::new(0, 0, 200, 100),
            fp_color::Rgba8::new(255, 255, 255, 255),
        )
        .unwrap();
        let pdf = crate::vector_out::write_pdf_with(
            &[crate::vector_out::PdfPage {
                image: &img,
                width_pt: 200.0,
                height_pt: 100.0,
                text: Some(&lay),
            }],
            "t",
            &crate::vector_out::PdfOptions {
                compression: crate::vector_out::PdfCompression::None,
                selectable_text: true,
                ..Default::default()
            },
        )
        .unwrap();
        let s = String::from_utf8_lossy(&pdf);
        assert!(s.contains("BT 3 Tr"));
        // TJ array: strings joined give the text; numbers are small position corrections
        // (hinted advances differ from the font's widths by under a pixel)
        let a = s.find("Tm [").expect("TJ") + 4;
        let b = s[a..].find("] TJ").expect("TJ end") + a;
        let mut text = String::new();
        let mut rest = &s[a..b];
        while let Some(o) = rest.find('(') {
            let c = rest[o..].find(')').expect(")") + o;
            text.push_str(&rest[o + 1..c]);
            for n in rest[c + 1..]
                .split('(')
                .next()
                .unwrap_or("")
                .split_whitespace()
            {
                let v: f64 = n.parse().expect("number");
                assert!(v.abs() < 60.0, "correction {v}");
            }
            rest = &rest[c + 1..];
        }
        assert_eq!(text, "Hello");
    }
}
