//! Anbindung des Exports (fp-export) an den Renderer der Sitzung.
//!
//! fp-export hängt nicht von fp-render ab, sondern rendert über die schmale Schnittstelle
//! [`fp_export::render::PageRenderer`]. [`ExportRenderer`] setzt sie mit dem Renderer der Sitzung
//! ([`crate::render::PageRenderer`], Vorgabe fp-render) um:
//!
//! * Ebenenfilter (`Only`/`Exclude`): auf einer billigen Kopie des Dokuments (Copy-on-Write) wird
//!   die Sichtbarkeit der Ebenen der Seite gesetzt („Ebenen in Dateien“ rendert auch unsichtbare
//!   Ebenen, PSD-Export je Ebene).
//! * Hintergrund: Leinwandfarbe, transparent (Export mit Matte) oder feste Farbe – über die
//!   Leinwandfarbe der Kopie.
//! * Ausschnitt (`area`): Seite rendern und zuschneiden (außerhalb der Leinwand transparent).
//!
//! Ohne Renderer der Sitzung greift [`fp_export::render::SimpleRenderer`] (Bitmaps und gespeicherte
//! Caches).

use crate::render::PageRenderer;
use fp_export::render::{Background, LayerFilter, RenderRequest, SimpleRenderer};
use fp_model::{Document, LayerSlot};
use fp_raster::RgbaImage;
use std::sync::Arc;

/// Renderer der Sitzung als `fp_export::render::PageRenderer`.
pub struct ExportRenderer<'a>(pub Option<&'a dyn PageRenderer>);

impl<'a> ExportRenderer<'a> {
    /// Adapter für den Renderer der Sitzung (bzw. den Rückfall).
    pub fn new(r: Option<&'a Arc<dyn PageRenderer>>) -> ExportRenderer<'a> {
        ExportRenderer(r.map(|a| a.as_ref()))
    }
}

/// Dokumentkopie, in der Ebenenfilter und Hintergrund eines Auftrags umgesetzt sind.
pub fn prepared_document(doc: &Document, req: &RenderRequest) -> Document {
    let mut d = doc.clone();
    if let Some(p) = d.pages.get_mut(req.page) {
        let p = Arc::make_mut(p);
        match req.background {
            Background::Canvas => {}
            Background::Transparent => p.root.background = fp_color::Argb(0x00FF_FFFF),
            Background::Color(c) => p.root.background = fp_color::Argb(c),
        }
        if req.layers != LayerFilter::All {
            for slot in p.root.layers.iter_mut() {
                if let LayerSlot::Own(l) = slot {
                    let want = match &req.layers {
                        LayerFilter::All => continue,
                        LayerFilter::Only(ids) => ids.contains(&l.id),
                        LayerFilter::Exclude(ids) => {
                            if !ids.contains(&l.id) {
                                continue;
                            }
                            false
                        }
                    };
                    let l = Arc::make_mut(l);
                    let n = l.visibility.len().max(req.frame + 1).max(1);
                    l.visibility = vec![want; n];
                    l.has_visibility_node = true;
                }
            }
        }
    }
    // Geteilte Ebenen (über Seiten) ebenfalls filtern
    if let LayerFilter::Only(_) | LayerFilter::Exclude(_) = &req.layers {
        for sl in d.shared_layers.iter_mut() {
            let Some(l) = sl.layer.as_mut() else { continue };
            let want = match &req.layers {
                LayerFilter::Only(ids) => ids.contains(&l.id),
                LayerFilter::Exclude(ids) => {
                    if !ids.contains(&l.id) {
                        continue;
                    }
                    false
                }
                LayerFilter::All => continue,
            };
            let l = Arc::make_mut(l);
            let n = l.visibility.len().max(req.frame + 1).max(1);
            l.visibility = vec![want; n];
        }
    }
    d
}

/// Schneidet `img` auf `area` zu; Bereiche außerhalb werden transparent ergänzt.
fn crop_to(img: RgbaImage, area: fp_geom::IRect) -> RgbaImage {
    if img.bounds() == area {
        return img;
    }
    match RgbaImage::new(area) {
        Ok(mut out) => {
            let src = img.cropped(area);
            if !src.is_empty() {
                fp_raster::composite(&mut out, &src, None, &fp_raster::CompositeOptions::default());
            }
            out
        }
        Err(_) => img.cropped(area),
    }
}

impl fp_export::render::PageRenderer for ExportRenderer<'_> {
    fn render(&self, doc: &Document, req: &RenderRequest) -> Result<RgbaImage, String> {
        if doc.pages.get(req.page).is_none() {
            return Err(fp_i18n::tr!("err-page-missing", n = (req.page + 1).to_string()));
        }
        let Some(r) = self.0 else {
            return SimpleRenderer.render(doc, req);
        };
        let d = prepared_document(doc, req);
        let img = r.render_page(&d, req.page, req.frame).ok_or_else(|| fp_i18n::t("err-renderer-unavailable"))?;
        Ok(match req.area {
            Some(a) => crop_to(img, a),
            None => img,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_export::render::PageRenderer as _;

    #[test]
    fn transparent_background_and_area() {
        let d = Document::new(10.0, 8.0, fp_color::Argb(0xFFFF_0000));
        let r = crate::render::default_renderer();
        let er = ExportRenderer::new(r.as_ref());
        let img = er.render(&d, &RenderRequest::default()).unwrap();
        assert_eq!((img.width(), img.height()), (10, 8));
        assert_eq!(img.pixel(3, 3).r, 255);
        let t = er.render(&d, &RenderRequest { background: Background::Transparent, area: Some(fp_geom::IRect::new(2, 2, 6, 6)), ..Default::default() }).unwrap();
        assert_eq!((t.width(), t.height()), (4, 4));
        assert_eq!(t.pixel(3, 3).a, 0);
    }
}
