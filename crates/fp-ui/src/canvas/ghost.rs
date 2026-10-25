//! Vorschau-Texturen der Leinwand: gezogene/transformierte Objekte („Vorschau beim Ziehen“) und
//! der bearbeitete Text. Die Elemente werden einmal mit fp-render (`render_element`, 1:1 in
//! Dokumentpixeln) gerendert und danach nur noch als texturiertes Gitter über die Leinwand gelegt
//! (affin exakt, projektiv mit 8×8-Unterteilung) – ruckelfreies Ziehen ohne Neurendern des
//! Dokuments.

use egui::{Color32, Mesh, Pos2, Rect, TextureHandle, TextureOptions, pos2};
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
    /// A pixel move (FB-73): [`GhostCache::pixels`] instead of [`GhostCache::drag`].
    pub pixels: bool,
}

/// Vorschau-Texturen eines Dokuments.
#[derive(Default)]
pub struct GhostCache {
    /// Je Lauf gezogener Objekte (unten → oben, [`super::stack::Run`]) eine Vorschau; leer = keine.
    pub drag: Vec<Option<Sprite>>,
    /// Schlüssel von `drag`.
    pub drag_key: u64,
    /// Ausschnitt (Dokumentpixel), auf den `drag` begrenzt ist; `None` = ganz (BA-097).
    pub drag_clip: Option<IRect>,
    /// Grenzen der ganzen Vorschau aller Läufe (auch außerhalb von `drag_clip`).
    pub drag_full: IRect,
    pub text: Option<Sprite>,
    /// Objekt mit live gezogenen Füllgriffen (Zeiger, re 40 §1).
    pub fill: Option<Sprite>,
    pub linger: Option<Linger>,
    /// Vorschau einer Pixelverschiebung (FB-73); `None` = keine.
    pub pixels: Option<PixelGhost>,
}

/// A preview texture limited to a part of its full extent (BA-097).
#[derive(Default)]
pub struct Clipped {
    pub sprite: Option<Sprite>,
    /// Part (document pixels) the texture is limited to; `None` = whole.
    pub clip: Option<IRect>,
    /// Bounds of the whole preview.
    pub full: IRect,
}

impl Clipped {
    /// Does the texture hold the part `need`?
    fn covers(&self, need: Option<IRect>) -> bool {
        clip_covers(self.clip, self.full, need)
    }
}

/// Live preview of a pixel move (FB-73): the bitmap with the selected pixels cut out (drawn in
/// place: what the committed move leaves behind) and the selected pixels alone (drawn at the
/// offset).
pub struct PixelGhost {
    /// Key (generation, bitmap, selection mask).
    pub key: u64,
    pub hole: Clipped,
    pub cluster: Clipped,
}

/// Does a texture limited to `clip` (of the whole `full`) hold the part `need`?
fn clip_covers(clip: Option<IRect>, full: IRect, need: Option<IRect>) -> bool {
    match (clip, need) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(c), Some(n)) => {
            let n = n.intersect(&full);
            n.is_empty() || c.intersect(&n) == n
        }
    }
}

/// The part a texture for `need` is cut to: `need` plus half its size on each side (BA-097).
fn clip_around(need: Option<IRect>) -> Option<IRect> {
    need.map(|n| {
        let (mx, my) = ((n.width() / 2) as i32, (n.height() / 2) as i32);
        IRect::new(n.left - mx, n.top - my, n.right + mx, n.bottom + my)
    })
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
fn texture_of(
    ctx: &egui::Context,
    name: &str,
    img: &fp_raster::RgbaImage,
    zoom: f64,
) -> TextureHandle {
    let opt = if zoom >= 1.0 {
        TextureOptions::NEAREST
    } else {
        TextureOptions::LINEAR
    };
    ctx.load_texture(name, super::render::to_color_image(img), opt)
}

impl GhostCache {
    /// Vorschauen der Läufe `runs` (je Lauf die gezogenen Elemente, unten → oben; Seite `page`,
    /// Status `frame`) – gecacht je Generation. Ein Lauf ohne sichtbaren Inhalt liefert `None`.
    ///
    /// `need`: der Teil der Vorschau (Dokumentpixel vor dem Ziehen), der gerade im Bild liegt.
    /// Die Textur hält nur ihn plus einen Rand (je Seite die halbe Größe); erst wenn `need` den
    /// Ausschnitt verlässt (Rollen, Zoomen, weites Ziehen), wird neu geschnitten. Ohne Grafikkarte
    /// lädt die Textur nur mit ~30 MB/s hoch, eine Vorschau von 6000 × 1500 px brauchte über 1 s
    /// (BA-097). Innerhalb des Ausschnitts sind die Pixel dieselben.
    #[allow(clippy::too_many_arguments)] // Kontext der Vorschau (egui, Renderer, Dokument, Lage)
    pub fn drag_sprites(
        &mut self,
        ctx: &egui::Context,
        r: &fp_render::Renderer,
        doc: &Document,
        generation: u64,
        page: usize,
        frame: usize,
        runs: &[Vec<u64>],
        zoom: f64,
        need: Option<IRect>,
    ) -> &[Option<Sprite>] {
        let flat: Vec<u64> = runs
            .iter()
            .flat_map(|ids| ids.iter().copied().chain(std::iter::once(u64::MAX)))
            .collect();
        let key = hash_ids(generation, &flat);
        let fresh = !self.drag.is_empty() && self.drag_key == key;
        let covered = clip_covers(self.drag_clip, self.drag_full, need);
        if !fresh || !covered {
            let clip = clip_around(need);
            let mut full = IRect::default();
            self.drag = runs
                .iter()
                .enumerate()
                .map(|(i, ids)| {
                    let els: Vec<Arc<Element>> = ids
                        .iter()
                        .filter_map(|id| {
                            fp_engine::docops::element_on_page(doc, page, ElementId(*id))
                        })
                        .collect();
                    let (img, f) = super::render::render_elements_in(r, doc, frame, &els, clip);
                    full = full.union(&f);
                    let img = img?;
                    Some(Sprite {
                        key,
                        texture: texture_of(ctx, &format!("canvas-ghost-{i}"), &img, zoom),
                        bounds: img.bounds(),
                    })
                })
                .collect();
            self.drag_key = key;
            self.drag_clip = clip;
            self.drag_full = full;
        }
        &self.drag
    }

    /// Live preview of a pixel move (FB-73) of the selected pixels `mask` of the bitmap `id`:
    /// (the bitmap with the hole, the selected pixels alone), each limited to the part in view
    /// (`need_hole`, `need_cluster`: document pixels before the move, see [`needed_part`]) plus a
    /// margin like [`Self::drag_sprites`]. Cached per generation, bitmap and mask. `copy`
    /// (Subselection drag, RE spec 71 section 6.2): the bitmap stays unchanged (no hole) and a
    /// copy of the selected pixels moves.
    #[allow(clippy::too_many_arguments)] // Kontext der Vorschau (egui, Renderer, Dokument, Lage)
    pub fn pixel_sprites(
        &mut self,
        ctx: &egui::Context,
        r: &fp_render::Renderer,
        doc: &Document,
        (generation, page, frame): (u64, usize, usize),
        (id, copy): (u64, bool),
        mask: &fp_raster::Mask,
        zoom: f64,
        (need_hole, need_cluster): (Option<IRect>, Option<IRect>),
    ) -> (Option<&Sprite>, Option<&Sprite>) {
        let mk = super::overlay::mask_key(mask);
        let key = hash_ids(
            generation,
            &[
                id,
                mk.0 as u64,
                mk.1 as u64,
                mk.2 as u64,
                mk.3 as u64,
                mk.4 as u64,
                copy as u64,
            ],
        );
        let fresh = self.pixels.as_ref().is_some_and(|p| p.key == key);
        let redo_hole = !fresh
            || !self
                .pixels
                .as_ref()
                .is_some_and(|p| p.hole.covers(need_hole));
        let redo_cluster = !fresh
            || !self
                .pixels
                .as_ref()
                .is_some_and(|p| p.cluster.covers(need_cluster));
        if redo_hole || redo_cluster {
            let mut pg = self.pixels.take().filter(|_| fresh).unwrap_or(PixelGhost {
                key,
                hole: Clipped::default(),
                cluster: Clipped::default(),
            });
            let e = fp_engine::docops::element_on_page(doc, page, ElementId(id));
            let split = e.as_ref().and_then(|e| match &e.kind {
                fp_model::ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b).map(|img| {
                    let (hole, cluster) = fp_engine::docops::cut_selected_pixels(&img, mask);
                    (if copy { img } else { hole }, cluster)
                }),
                _ => None,
            });
            let render = |img: &fp_raster::RgbaImage, need: Option<IRect>, name: &str| {
                let mut c = Clipped::default();
                let Some(e) = e.as_ref() else { return c };
                let mut el = (**e).clone();
                if let fp_model::ElementKind::Bitmap(b) = &mut el.kind {
                    fp_engine::docops::set_bitmap_image(b, img);
                }
                let clip = clip_around(need);
                let (out, full) =
                    super::render::render_elements_in(r, doc, frame, &[Arc::new(el)], clip);
                c.clip = clip;
                c.full = full;
                c.sprite = out.map(|img| Sprite {
                    key,
                    texture: texture_of(ctx, name, &img, zoom),
                    bounds: img.bounds(),
                });
                c
            };
            if let Some((hole, cluster)) = &split {
                if redo_hole {
                    pg.hole = render(hole, need_hole, "canvas-ghost-pixels-hole");
                }
                if redo_cluster {
                    pg.cluster = if cluster.width() > 0 && cluster.height() > 0 {
                        render(cluster, need_cluster, "canvas-ghost-pixels")
                    } else {
                        Clipped::default()
                    };
                }
            } else {
                pg.hole = Clipped::default();
                pg.cluster = Clipped::default();
            }
            pg.key = key;
            self.pixels = Some(pg);
        }
        match &self.pixels {
            Some(p) => (p.hole.sprite.as_ref(), p.cluster.sprite.as_ref()),
            None => (None, None),
        }
    }

    /// Vorschau eines Elements mit live gezogenen Füllgriffen; `key` beschreibt den Inhalt.
    #[allow(clippy::too_many_arguments)]
    pub fn fill_sprite(
        &mut self,
        ctx: &egui::Context,
        r: &fp_render::Renderer,
        doc: &Document,
        frame: usize,
        e: &Element,
        key: u64,
        zoom: f64,
    ) -> Option<&Sprite> {
        if self.fill.as_ref().is_none_or(|s| s.key != key) {
            let map = fp_render::PointMap::identity();
            let img = r
                .render_element(doc, &Arc::new(e.clone()), &map, frame)
                .image;
            self.fill = (img.width() > 0).then(|| Sprite {
                key,
                texture: texture_of(ctx, "canvas-fill-live", &img, zoom),
                bounds: img.bounds(),
            });
        }
        self.fill.as_ref()
    }

    /// Vorschau eines (Text-)Elements; `key` beschreibt den Inhalt.
    #[allow(clippy::too_many_arguments)]
    pub fn element_sprite(
        &mut self,
        ctx: &egui::Context,
        r: &fp_render::Renderer,
        doc: &Document,
        frame: usize,
        e: &Element,
        key: u64,
        zoom: f64,
    ) -> Option<&Sprite> {
        if self.text.as_ref().is_none_or(|s| s.key != key) {
            let map = fp_render::PointMap::identity();
            let img = r
                .render_element(doc, &Arc::new(e.clone()), &map, frame)
                .image;
            if img.width() == 0 {
                self.text = None;
                return None;
            }
            self.text = Some(Sprite {
                key,
                texture: texture_of(ctx, "canvas-text", &img, zoom),
                bounds: img.bounds(),
            });
        }
        self.text.as_ref()
    }
}

/// Teil einer Vorschau (Dokumentpixel vor dem Ziehen), der im Sichtbereich `visible`
/// (Dokumentkoordinaten) landet, wenn `rect` auf `quad` abgebildet wird (BA-097); `None` bei
/// projektiver Abbildung (dann ganz).
pub fn needed_part(
    rect: fp_geom::Rect,
    quad: &[Point; 4],
    visible: fp_geom::Rect,
) -> Option<IRect> {
    let m = Matrix3::rect_to_quad(&rect, quad)?;
    if !m.is_affine() {
        return None;
    }
    let inv = m.inverse()?;
    let corners = [
        Point::new(visible.left, visible.top),
        Point::new(visible.right, visible.top),
        Point::new(visible.left, visible.bottom),
        Point::new(visible.right, visible.bottom),
    ]
    .map(|p| inv.apply(p));
    let (mut l, mut t, mut r, mut b) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in corners {
        if !p.x.is_finite() || !p.y.is_finite() {
            return None;
        }
        (l, t, r, b) = (l.min(p.x), t.min(p.y), r.max(p.x), b.max(p.y));
    }
    let lim = 1.0e7;
    Some(IRect::new(
        (l.floor().max(-lim) as i32) - 1,
        (t.floor().max(-lim) as i32) - 1,
        (r.ceil().min(lim) as i32) + 1,
        (b.ceil().min(lim) as i32) + 1,
    ))
}

/// Zeichnet eine Vorschau: Dokumentrechteck `rect` wird auf das Viereck `quad` abgebildet;
/// `to_screen` rechnet Dokument- in Bildschirmkoordinaten.
pub fn paint_mapped(
    painter: &egui::Painter,
    sprite: &Sprite,
    rect: fp_geom::Rect,
    quad: &[Point; 4],
    tint: Color32,
    to_screen: &dyn Fn(Point) -> Pos2,
) {
    let Some(m) = Matrix3::rect_to_quad(&rect, quad) else {
        return;
    };
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
            mesh.vertices.push(egui::epaint::Vertex {
                pos: to_screen(q),
                uv: pos2(u, v),
                color: tint,
            });
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
    painter.image(
        sprite.texture.id(),
        Rect::from_min_max(a, c),
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );
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
