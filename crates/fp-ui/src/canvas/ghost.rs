//! Vorschau-Texturen der Leinwand: gezogene/transformierte Objekte („Vorschau beim Ziehen“) und
//! der bearbeitete Text. Die Elemente werden einmal mit fp-render (`render_element`, 1:1 in
//! Dokumentpixeln) gerendert und danach nur noch als texturiertes Gitter über die Leinwand gelegt
//! (affin exakt, projektiv mit 8×8-Unterteilung) – ruckelfreies Ziehen ohne Neurendern des
//! Dokuments.

use egui::{pos2, Color32, Mesh, Pos2, Rect, TextureHandle, TextureOptions};
use fp_geom::{IRect, Matrix3, Point};
use fp_model::{Document, Element, ElementId};
use std::sync::Arc;

/// Eine gerenderte Vorschau.
pub struct Sprite {
    /// Schlüssel (Generation, Inhalt).
    pub key: u64,
    pub texture: TextureHandle,
    /// Lage in Dokumentpixeln.
    pub bounds: IRect,
}

/// Vorschau nach dem Übernehmen, bis das neue Hintergrundbild da ist (kein Flackern).
pub struct Linger {
    /// Generation beim letzten Ziehen.
    pub generation: u64,
    pub rect: fp_geom::Rect,
    pub quad: [Point; 4],
    pub ids: Vec<u64>,
}

/// Vorschau-Texturen eines Dokuments.
#[derive(Default)]
pub struct GhostCache {
    pub drag: Option<Sprite>,
    pub text: Option<Sprite>,
    pub linger: Option<Linger>,
}

fn hash_ids(generation: u64, ids: &[u64]) -> u64 {
    let mut h: u64 = 0x84222325cbf29ce4 ^ generation.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    for id in ids {
        h ^= *id;
        h = h.wrapping_mul(0x0100_0000_01b3).rotate_left(7);
    }
    h
}

/// Textur aus einem RGBA-Bild.
fn texture_of(ctx: &egui::Context, name: &str, img: &fp_raster::RgbaImage, zoom: f64) -> TextureHandle {
    let opt = if zoom >= 1.0 { TextureOptions::NEAREST } else { TextureOptions::LINEAR };
    ctx.load_texture(name, super::render::to_color_image(img), opt)
}

impl GhostCache {
    /// Vorschau der Elemente `ids` (Seite `page`, Status `frame`) – gecacht je Generation.
    #[allow(clippy::too_many_arguments)] // Kontext der Vorschau (egui, Renderer, Dokument, Lage)
    pub fn drag_sprite(&mut self, ctx: &egui::Context, r: &fp_render::Renderer, doc: &Document, generation: u64, page: usize, frame: usize, ids: &[u64], zoom: f64) -> Option<&Sprite> {
        let key = hash_ids(generation, ids);
        if self.drag.as_ref().is_none_or(|s| s.key != key) {
            let els: Vec<Arc<Element>> = ids.iter().filter_map(|id| fp_engine::docops::element_on_page(doc, page, ElementId(*id))).collect();
            let img = super::render::render_elements(r, doc, frame, &els)?;
            self.drag = Some(Sprite { key, texture: texture_of(ctx, "canvas-ghost", &img, zoom), bounds: img.bounds() });
        }
        self.drag.as_ref()
    }

    /// Vorschau eines (Text-)Elements; `key` beschreibt den Inhalt.
    #[allow(clippy::too_many_arguments)]
    pub fn element_sprite(&mut self, ctx: &egui::Context, r: &fp_render::Renderer, doc: &Document, frame: usize, e: &Element, key: u64, zoom: f64) -> Option<&Sprite> {
        if self.text.as_ref().is_none_or(|s| s.key != key) {
            let map = fp_render::PointMap::identity();
            let img = r.render_element(doc, &Arc::new(e.clone()), &map, frame).image;
            if img.width() == 0 {
                self.text = None;
                return None;
            }
            self.text = Some(Sprite { key, texture: texture_of(ctx, "canvas-text", &img, zoom), bounds: img.bounds() });
        }
        self.text.as_ref()
    }
}

/// Zeichnet eine Vorschau: Dokumentrechteck `rect` wird auf das Viereck `quad` abgebildet;
/// `to_screen` rechnet Dokument- in Bildschirmkoordinaten.
pub fn paint_mapped(painter: &egui::Painter, sprite: &Sprite, rect: fp_geom::Rect, quad: &[Point; 4], tint: Color32, to_screen: &dyn Fn(Point) -> Pos2) {
    let Some(m) = Matrix3::rect_to_quad(&rect, quad) else { return };
    let b = sprite.bounds;
    let affine = m.is_affine();
    let n = if affine { 1 } else { 8 };
    let mut mesh = Mesh::with_texture(sprite.texture.id());
    let (w, h) = (b.width() as f32, b.height() as f32);
    for j in 0..=n {
        for i in 0..=n {
            let (u, v) = (i as f32 / n as f32, j as f32 / n as f32);
            let p = Point::new(b.left as f32 + u * w, b.top as f32 + v * h);
            let q = m.apply(p);
            mesh.vertices.push(egui::epaint::Vertex { pos: to_screen(q), uv: pos2(u, v), color: tint });
        }
    }
    let stride = n + 1;
    for j in 0..n {
        for i in 0..n {
            let a = (j * stride + i) as u32;
            let bb = a + 1;
            let c = a + stride as u32;
            let d = c + 1;
            mesh.add_triangle(a, bb, d);
            mesh.add_triangle(a, d, c);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// Zeichnet eine Vorschau an ihrer eigenen Lage.
pub fn paint_at(painter: &egui::Painter, sprite: &Sprite, to_screen: &dyn Fn(Point) -> Pos2) {
    let b = sprite.bounds;
    let a = to_screen(Point::new(b.left as f32, b.top as f32));
    let c = to_screen(Point::new(b.right as f32, b.bottom as f32));
    painter.image(sprite.texture.id(), Rect::from_min_max(a, c), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_hash_depends_on_generation_and_ids() {
        assert_ne!(hash_ids(1, &[1, 2]), hash_ids(2, &[1, 2]));
        assert_ne!(hash_ids(1, &[1, 2]), hash_ids(1, &[2, 1]));
    }
}
