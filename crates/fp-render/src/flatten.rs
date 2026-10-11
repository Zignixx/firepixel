//! Anbindung an den Schreiber: [`fp_fwpng::Flattener`] für das flache Bild (`IDAT`) und die
//! Vorschau (`prVW`).
//!
//! `flatten` rendert die Seite im Status 0 mit Leinwandfarbe. `render_scaled` liefert die Miniatur
//! nach 02 §12.8: Ursprung `(⌊L·s2⌋, ⌊T·s2⌋)`, je Zielpixel der nächste Nachbar der vollen
//! Auflösung (Laufzeit 02 T3: Bitmaps per Punktabtastung exakt; Vektor- und Textkanten weichen
//! leicht ab, weil das Original sie im Maßstab neu rastert – A).

use fp_fwpng::Flattener;
use fp_model::{Document, FlatImage};

use crate::compose::Renderer;

impl Flattener for Renderer {
    fn flatten(&self, doc: &Document, page: usize) -> Option<FlatImage> {
        let out = self.render_page(doc, page, 0);
        let img = out.image;
        if img.is_empty() {
            return None;
        }
        Some(FlatImage { width: img.width() as u32, height: img.height() as u32, rgba: img.as_bytes().to_vec() })
    }

    fn render_scaled(&self, doc: &Document, page: usize, scale: f64, width: u32, height: u32) -> Option<FlatImage> {
        if !(scale.is_finite() && scale > 0.0) || width == 0 || height == 0 {
            return None;
        }
        let rect = Renderer::page_rect(doc, page)?;
        let full = self.render_page(doc, page, 0).image;
        let ox = (rect.left as f64 * scale).floor();
        let oy = (rect.top as f64 * scale).floor();
        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
        for j in 0..height {
            let y = (((oy + j as f64) + 0.5) / scale).floor() as i32;
            for i in 0..width {
                let x = (((ox + i as f64) + 0.5) / scale).floor() as i32;
                rgba.extend_from_slice(&full.get(x, y).unwrap_or([0, 0, 0, 0]));
            }
        }
        Some(FlatImage { width, height, rgba })
    }
}
