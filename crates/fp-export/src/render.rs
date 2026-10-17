//! Schmale Renderer-Schnittstelle: „Dokument flach rechnen“ für den Export.
//!
//! `fp-export` hängt **nicht** von `fp-render` ab. Wer exportiert (fp-engine, CLI, UI), übergibt einen
//! [`PageRenderer`] — typischerweise einen Adapter auf den Compositor von `fp-render`:
//!
//! ```ignore
//! struct FpRender;
//! impl fp_export::render::PageRenderer for FpRender {
//!     fn render(&self, doc: &fp_model::Document, req: &fp_export::render::RenderRequest)
//!         -> Result<fp_raster::RgbaImage, String> {
//!         // fp_render::… (Seite req.page, Status req.frame, Ebenenfilter req.layers)
//!         # unimplemented!()
//!     }
//! }
//! ```
//!
//! Die Umsetzung mit dem echten Compositor ist `fp_engine::export_backend::ExportRenderer`
//! (Renderer der Sitzung, Ebenenfilter über die Ebenen-Sichtbarkeit einer Dokumentkopie,
//! Hintergrund über die Leinwandfarbe, Ausschnitt durch Zuschneiden); fp-engine benutzt sie für alle
//! Export-Befehle.
//!
//! Für Tests und als Rückfall gibt es [`SimpleRenderer`]: Er setzt Bitmaps, Effekt-Ergebnisse und die
//! gespeicherten Render-Caches (Pfade, Texte, Gruppen, Instanzen) mit Deckkraft und Mischmodus
//! zusammen, ohne Vektoren neu zu rastern oder Live-Effekte zu rechnen.

use std::sync::Arc;

use fp_color::{BlendMode, Opacity};
use fp_geom::IRect;
use fp_model::{Document, Element, ElementKind, Layer, LayerId, LayerKind, LayerSlot};
use fp_raster::{CompositeOptions, RgbaImage, composite};

/// Welche Ebenen gerendert werden.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum LayerFilter {
    /// Alle sichtbaren Ebenen.
    #[default]
    All,
    /// Nur diese Ebenen (auch wenn sie unsichtbar sind; „Ebenen in Dateien“).
    Only(Vec<LayerId>),
    /// Alle sichtbaren außer diesen.
    Exclude(Vec<LayerId>),
}

/// Hintergrund des Ergebnisses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Background {
    /// Leinwandfarbe der Seite (Original-Export ohne Matte, 08 §2.1).
    #[default]
    Canvas,
    /// Transparent (`0x00FFFFFF`, Export mit gesetzter Matte).
    Transparent,
    /// Feste Farbe `0xAARRGGBB`.
    Color(u32),
}

/// Auftrag an den Renderer.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RenderRequest {
    /// Seitenindex.
    pub page: usize,
    /// Status (0-basiert).
    pub frame: usize,
    pub layers: LayerFilter,
    pub background: Background,
    /// Ausschnitt in Dokumentkoordinaten (`None` = ganze Leinwand).
    pub area: Option<IRect>,
}

/// Rechnet eine Seite (bzw. einen Ausschnitt) flach. Das Ergebnis hat Dokumentkoordinaten
/// (Grenzen = Leinwand bzw. `area`).
pub trait PageRenderer {
    fn render(&self, doc: &Document, req: &RenderRequest) -> Result<RgbaImage, String>;
}

impl<F> PageRenderer for F
where
    F: Fn(&Document, &RenderRequest) -> Result<RgbaImage, String>,
{
    fn render(&self, doc: &Document, req: &RenderRequest) -> Result<RgbaImage, String> {
        self(doc, req)
    }
}

/// Leinwandrechteck einer Seite in Dokumentkoordinaten.
pub fn page_rect(doc: &Document, page: usize) -> Option<IRect> {
    let p = doc.pages.get(page)?;
    let x = p.canvas.origin.x.floor() as i32;
    let y = p.canvas.origin.y.floor() as i32;
    Some(IRect::new(
        x,
        y,
        x + p.canvas.width.round().max(0.0) as i32,
        y + p.canvas.height.round().max(0.0) as i32,
    ))
}

/// Rückfall-Renderer (Bitmaps und gespeicherte Caches, siehe Moduldoku).
#[derive(Clone, Copy, Debug, Default)]
pub struct SimpleRenderer;

impl SimpleRenderer {
    fn draw_bitmap(
        dst: &mut RgbaImage,
        node_img: &fp_model::ImageData,
        opacity: Opacity,
        blend: BlendMode,
    ) {
        let (rgba, _) = node_img.to_rgba();
        let pos = node_img.position();
        let (x, y) = (pos.x.round() as i32, pos.y.round() as i32);
        let r = IRect::new(
            x,
            y,
            x + node_img.grid.width as i32,
            y + node_img.grid.height as i32,
        );
        if let Ok(src) = RgbaImage::from_rgba_bytes(r, &rgba) {
            composite(
                dst,
                &src,
                None,
                &CompositeOptions {
                    mode: blend,
                    opacity,
                    ..CompositeOptions::default()
                },
            );
        }
    }

    fn draw_element(dst: &mut RgbaImage, e: &Element, frame: usize, depth: u32) {
        if !e.common.visible || depth > 64 {
            return;
        }
        let (op, bl) = (e.common.opacity, e.common.blend);
        if let Some(c) = &e.cache
            && !matches!(e.kind, ElementKind::Path(_) | ElementKind::Bitmap(_))
        {
            Self::draw_bitmap(dst, &c.bitmap.image, op, bl);
            return;
        }
        match &e.kind {
            ElementKind::Bitmap(b) => {
                let img = b
                    .effect_result
                    .as_ref()
                    .map(|r| &r.bitmap.image)
                    .unwrap_or(&b.image);
                Self::draw_bitmap(dst, img, op, bl);
            }
            ElementKind::Path(_) => {
                if let Some(c) = &e.cache {
                    Self::draw_bitmap(dst, &c.bitmap.image, op, bl);
                }
            }
            ElementKind::Group(g) => {
                for c in g.children.iter().rev() {
                    Self::draw_element(dst, c, frame, depth + 1);
                }
            }
            ElementKind::SubLayer(l) => Self::draw_layer(dst, l, frame, depth + 1),
            _ => {}
        }
    }

    fn draw_layer(dst: &mut RgbaImage, l: &Arc<Layer>, frame: usize, depth: u32) {
        let cel = if depth == 0 {
            l.cel(frame)
        } else {
            l.cels.first()
        };
        if let Some(cel) = cel {
            for e in cel.elements.iter().rev() {
                Self::draw_element(dst, e, frame, depth);
            }
        }
    }
}

impl PageRenderer for SimpleRenderer {
    fn render(&self, doc: &Document, req: &RenderRequest) -> Result<RgbaImage, String> {
        let page = doc
            .pages
            .get(req.page)
            .ok_or_else(|| format!("Seite {} fehlt", req.page))?;
        let rect = req
            .area
            .or_else(|| page_rect(doc, req.page))
            .ok_or("keine Leinwand")?;
        let bg = match req.background {
            Background::Canvas => page.root.background.0,
            Background::Transparent => 0x00FF_FFFF,
            Background::Color(c) => c,
        };
        let mut img =
            RgbaImage::filled(rect, fp_color::Argb(bg).to_rgba8()).map_err(|e| e.to_string())?;
        for slot in &page.root.layers {
            let layer: Option<Arc<Layer>> = match slot {
                LayerSlot::Own(l) => Some(l.clone()),
                LayerSlot::Shared(r) => doc
                    .shared_layers
                    .iter()
                    .find(|d| d.id == r.id)
                    .and_then(|d| d.layer.clone()),
                _ => None,
            };
            let Some(l) = layer else { continue };
            if l.kind == LayerKind::Web {
                continue;
            }
            let wanted = match &req.layers {
                LayerFilter::All => l.is_visible(),
                LayerFilter::Only(ids) => ids.contains(&l.id),
                LayerFilter::Exclude(ids) => l.is_visible() && !ids.contains(&l.id),
            };
            if wanted {
                Self::draw_layer(&mut img, &l, req.frame, 0);
            }
        }
        Ok(img)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_model::{Element, Point};

    #[test]
    fn simple_renderer_draws_bitmaps() {
        let mut d = Document::new(4.0, 4.0, fp_color::Argb(0xFFFF_FFFF));
        let mut ids = std::mem::take(&mut d.ids);
        let mut tid = 0;
        let e = Element::new_bitmap(
            &mut ids,
            Point::new(1.0, 1.0),
            2,
            2,
            &[255, 0, 0, 255].repeat(4),
            &mut tid,
        );
        d.ids = ids;
        if let Some(LayerSlot::Own(l)) = d.page_mut(0).map(|p| &mut p.root.layers[0]) {
            Arc::make_mut(&mut Arc::make_mut(l).cels[0])
                .elements
                .push(Arc::new(e));
        }
        let img = SimpleRenderer
            .render(&d, &RenderRequest::default())
            .unwrap();
        assert_eq!(img.pixel(0, 0), fp_color::Rgba8::new(255, 255, 255, 255));
        assert_eq!(img.pixel(1, 1), fp_color::Rgba8::new(255, 0, 0, 255));
        let t = SimpleRenderer
            .render(
                &d,
                &RenderRequest {
                    background: Background::Transparent,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(t.pixel(0, 0).a, 0);
    }
}
