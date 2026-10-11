//! Anbindung des Compositors fp-render (Feature `render`).
//!
//! Einzige Stelle, die die API von fp-render benutzt. Ohne das Feature bleibt die Sitzung ohne
//! Renderer; die UI kann jederzeit einen eigenen [`PageRenderer`] einstecken.

use crate::render::PageRenderer;
use fp_model::{Document, Element};
use fp_raster::RgbaImage;
use std::sync::Arc;

/// fp-render als [`PageRenderer`] (mit Element-Cache des Renderers).
#[derive(Default)]
pub struct FpRenderer(pub fp_render::Renderer);

impl PageRenderer for FpRenderer {
    fn render_page(&self, doc: &Document, page: usize, frame: usize) -> Option<RgbaImage> {
        Some(self.0.render_page(doc, page, frame).image)
    }

    fn render_elements(&self, doc: &Document, _page: usize, frame: usize, elements: &[Arc<Element>]) -> Option<RgbaImage> {
        let mut out: Option<RgbaImage> = None;
        let map = fp_render::PointMap::identity();
        for e in elements.iter().rev() {
            let img = self.0.render_element(doc, e, &map, frame).image;
            if img.is_empty() {
                continue;
            }
            let opt = fp_raster::CompositeOptions::default();
            out = Some(match out {
                None => img,
                Some(o) => fp_raster::composite_extend(&o, &img, None, &opt),
            });
        }
        out
    }

    fn flattener(&self) -> Option<&dyn fp_fwpng::Flattener> {
        Some(&self.0)
    }

    fn text_outlines(&self, doc: &Document, t: &fp_model::TextElem) -> Option<Vec<Vec<fp_geom::BezierContour>>> {
        let block = fp_render::convert::text_block(doc, t);
        let mut lay = fp_text::layout(&block, self.0.fonts(), &fp_text::LayoutOptions::default());
        if let Some(a) = &t.transform
            && !a.is_identity()
        {
            let aff = fp_text::raster::Affine {
                a: a.m[0][0] as f64,
                b: a.m[0][1] as f64,
                c: a.m[1][0] as f64,
                d: a.m[1][1] as f64,
                e: a.m[2][0] as f64,
                f: a.m[2][1] as f64,
            };
            for g in &mut lay.glyphs {
                g.transform = g.transform.then(&aff);
            }
            for d in &mut lay.decorations {
                for q in &mut d.quad {
                    *q = aff.apply(*q);
                }
            }
        }
        Some(fp_text::render::glyph_contours(&lay))
    }
}
