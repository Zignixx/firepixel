//! Schnittstelle zum Dokument-Compositor (fp-render).
//!
//! fp-engine rendert nicht selbst. Befehle, die das gerenderte Aussehen brauchen (Pipette im
//! Kompositmodus, „Auswahl auf Hintergrund reduzieren“ mit Vektoren, flaches Bild beim Speichern),
//! fragen einen [`PageRenderer`] an, den die Anwendung in die Sitzung steckt
//! ([`crate::session::Session::set_renderer`]). Ohne Renderer arbeiten diese Befehle mit Bitmaps
//! direkt bzw. melden [`crate::EngineError::BackendMissing`].

use fp_model::{Document, Element, FlatImage};
use fp_raster::RgbaImage;
use std::sync::Arc;

/// Rendert Seiten und Elementgruppen eines Dokuments.
pub trait PageRenderer: Send + Sync {
    /// Seite `page` im Status `frame` in Leinwandgröße (Dokumentkoordinaten, Leinwandfarbe
    /// eingerechnet). `None` = nicht verfügbar.
    fn render_page(&self, doc: &Document, page: usize, frame: usize) -> Option<RgbaImage>;

    /// Nur die angegebenen Elemente (oben → unten) über transparentem Grund, Grenzen = deren
    /// Aussehen in Dokumentkoordinaten.
    fn render_elements(
        &self,
        doc: &Document,
        page: usize,
        frame: usize,
        elements: &[Arc<Element>],
    ) -> Option<RgbaImage>;

    /// Eigener `fp_fwpng::Flattener` für flaches Bild und Vorschau beim Speichern (Vorgabe: keiner,
    /// dann [`RendererFlattener`]).
    fn flattener(&self) -> Option<&dyn fp_fwpng::Flattener> {
        None
    }

    /// Glyphenkonturen eines Textes in Dokumentkoordinaten (je Glyphe eine Liste) für „In Pfade
    /// konvertieren“. `None` = keine Textauslegung verfügbar.
    fn text_outlines(
        &self,
        _doc: &Document,
        _text: &fp_model::TextElem,
    ) -> Option<Vec<Vec<fp_geom::BezierContour>>> {
        None
    }

    /// Layout-Rahmen eines Textes (`LFTf TOPf RITf BOTf`, vor `MTXv`) aus dem echten Satz. Die
    /// Sitzung setzt damit `TextElem::bounds` nach jeder Text-/Attributänderung (Auswahlrahmen,
    /// Treffertest, Ausrichten, Info). `None` = keine Textauslegung verfügbar.
    fn text_frame(&self, _doc: &Document, _text: &fp_model::TextElem) -> Option<fp_geom::Rect> {
        None
    }
}

/// Satzrahmen über fp-render/fp-text (für alle Renderer mit fp-render gleich).
#[cfg(feature = "render")]
pub fn fp_text_frame(
    fonts: &fp_text::FontDb,
    doc: &Document,
    t: &fp_model::TextElem,
) -> Option<fp_geom::Rect> {
    let block = fp_render::convert::text_block(doc, t);
    let lay = fp_text::layout(&block, fonts, &fp_text::LayoutOptions::default());
    let f = lay.frame;
    (f.right > f.left || f.bottom > f.top).then_some(f)
}

/// Vorgabe-Renderer der Sitzung: mit Feature `render` fp-render, sonst keiner.
pub fn default_renderer() -> Option<Arc<dyn PageRenderer>> {
    #[cfg(feature = "render")]
    {
        Some(Arc::new(crate::render_backend::FpRenderer::default()))
    }
    #[cfg(not(feature = "render"))]
    {
        None
    }
}

/// Adapter: ein [`PageRenderer`] als `fp_fwpng::Flattener` (flaches Bild und Vorschau beim Speichern).
pub struct RendererFlattener<'a>(pub &'a dyn PageRenderer);

impl fp_fwpng::Flattener for RendererFlattener<'_> {
    fn flatten(&self, doc: &Document, page: usize) -> Option<FlatImage> {
        let img = self.0.render_page(doc, page, 0)?;
        Some(FlatImage {
            width: img.width() as u32,
            height: img.height() as u32,
            rgba: img.as_bytes().to_vec(),
        })
    }

    fn render_scaled(
        &self,
        doc: &Document,
        page: usize,
        scale: f64,
        width: u32,
        height: u32,
    ) -> Option<FlatImage> {
        let img = self.0.render_page(doc, page, 0)?;
        let small = fp_raster::resample::resize(&img, width, height, fp_raster::Sampler::Nearest);
        let _ = scale;
        Some(FlatImage {
            width: small.width() as u32,
            height: small.height() as u32,
            rgba: small.as_bytes().to_vec(),
        })
    }
}
