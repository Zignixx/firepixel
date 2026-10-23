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

    /// Layout frame and set boxes of a text for folding a scale into its attributes
    /// ([`crate::docops::fold_text_scale`], RE spec 66 §3.3). Default: the layout frame for both.
    fn text_fit_boxes(&self, doc: &Document, text: &fp_model::TextElem) -> Option<TextFitBoxes> {
        self.text_frame(doc, text).map(|f| {
            let b = [f.left as f64, f.top as f64, f.right as f64, f.bottom as f64];
            TextFitBoxes {
                frame: f,
                lines: b,
                transform_box: b,
            }
        })
    }
}

/// Boxes of a laid-out text (before `MTXv`) for [`crate::docops::fold_text_scale`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextFitBoxes {
    /// Layout frame (`LFTf TOPf RITf BOTf`, what `TextElem::bounds` holds).
    pub frame: fp_geom::Rect,
    /// Set box, unrounded: left = leftmost line start, right = end of the widest line (sum of the
    /// rounded advances), top/bottom = the exact frame top and bottom (ascent above the first
    /// baseline, descent below the last one).
    pub lines: [f64; 4],
    /// Set box the transform frame of the Scale/Skew/Distort tools is built from (RE spec 67
    /// §2.1–2.4; [`crate::docops::transform_frame`]): point text = the layout frame; fixed-width
    /// text = a box around the set content, horizontally per line floor(min(line start, ink)) to
    /// ceil(max(line start + width, ink)) (the width counts a space at a wrap), vertically
    /// floor(first baseline − ascent) to ceil(last baseline + descent) with the unrounded first
    /// baseline. Whole pixels.
    pub transform_box: [f64; 4],
}

/// Frame and set boxes over fp-render/fp-text (the same for every renderer with fp-render).
#[cfg(feature = "render")]
pub fn fp_text_fit_boxes(
    fonts: &fp_text::FontDb,
    doc: &Document,
    t: &fp_model::TextElem,
) -> Option<TextFitBoxes> {
    let block = fp_render::convert::text_block(doc, t);
    let lay = fp_text::layout(&block, fonts, &fp_text::LayoutOptions::default());
    let f = lay.frame;
    if f.right <= f.left && f.bottom <= f.top {
        return None;
    }
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for l in &lay.lines {
        if l.width > 0.0 {
            lo = lo.min(l.start);
            hi = hi.max(l.start + l.width);
        }
    }
    if !(lo.is_finite() && hi.is_finite()) {
        (lo, hi) = (lay.frame_exact[0], lay.frame_exact[2]);
    }
    let frame = [f.left as f64, f.top as f64, f.right as f64, f.bottom as f64];
    let transform_box = if t.frame_type == Some(1) && !lay.orientation.is_vertical() {
        fixed_transform_box(&lay).unwrap_or(frame)
    } else {
        frame
    };
    Some(TextFitBoxes {
        frame: f,
        lines: [lo, lay.frame_exact[1], hi, lay.frame_exact[3]],
        transform_box,
    })
}

/// Set box of a fixed-width text for the transform frame (RE spec 67 §2.4, measured for Arial,
/// left-aligned, leading 100 %): per line with content the advance box (line start … start + sum
/// of the rounded advances, a space at a wrap included) united with the ink, floor/ceil to whole
/// pixels; top = floor(unrounded first baseline − ascent), bottom = ceil(last baseline + descent).
#[cfg(feature = "render")]
fn fixed_transform_box(lay: &fp_text::TextLayout) -> Option<[f64; 4]> {
    let (first, last) = (lay.lines.first()?, lay.lines.last()?);
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for l in &lay.lines {
        if l.width <= 0.0 {
            continue;
        }
        let (mut a, mut b) = (l.start, l.start + l.width);
        for g in lay.glyphs.get(l.glyphs.clone()).unwrap_or_default() {
            if g.is_tab || !g.visible {
                continue;
            }
            if let Some(r) = g.ink_bounds() {
                a = a.min(r.left as f64);
                b = b.max(r.right as f64);
            }
        }
        lo = lo.min((a + 1e-4).floor());
        hi = hi.max((b - 1e-4).ceil());
    }
    if !(lo.is_finite() && hi.is_finite()) {
        return None;
    }
    let top = (first.baseline - first.ascent + 1e-6).floor();
    let bottom = (last.baseline + last.descent - 1e-6).ceil();
    (bottom > top).then_some([lo, top, hi, bottom])
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
