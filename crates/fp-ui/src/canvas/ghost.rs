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

// ------------------------------------------------- FB-87: pixel move through the element pipeline

/// Largest bitmap (pixels) whose pixel move previews through the page pipeline; larger ones keep
/// the two sprites of FB-73 (PERF-3 fallback, F141).
pub const LIVE_PIXELS_MAX: f64 = 16.0e6;

/// Does a pixel move of this element preview through the page pipeline (a render-only copy of
/// the document with the pixels moved)? FB-87 needed it for opacity, blend modes and live effects
/// (two sprites applied them twice); since F141 (RE spec 76 §7.1) every bitmap up to
/// [`LIVE_PIXELS_MAX`] uses it, so objects above with a blend mode compose right as well.
pub fn pixel_move_needs_pipeline(doc: &Document, e: &Element) -> bool {
    matches!(e.kind, fp_model::ElementKind::Bitmap(_))
        && fp_engine::docops::element_bounds(doc, e)
            .is_some_and(|b| (b.width() as f64) * (b.height() as f64) <= LIVE_PIXELS_MAX)
}

/// A live pixel move that renders through the element pipeline (FB-87): the bitmap, the offset
/// (whole pixels, like the commit) and whether a copy moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LivePixels {
    pub id: ElementId,
    pub dx: i32,
    pub dy: i32,
    pub copy: bool,
}

/// The pixel move in the tool feedback that needs the element pipeline
/// ([`pixel_move_needs_pipeline`]); `None` for none or one the sprites show exactly.
pub fn live_pixels(
    feedback: &[fp_engine::tools::Overlay],
    doc: &Document,
    page: usize,
) -> Option<LivePixels> {
    feedback.iter().find_map(|o| match o {
        fp_engine::tools::Overlay::Ghost {
            ids,
            rect,
            quad,
            copy,
            pixels: true,
            ..
        } => {
            let id = ElementId(*ids.first()?);
            let e = fp_engine::docops::element_on_page(doc, page, id)?;
            pixel_move_needs_pipeline(doc, &e).then(|| LivePixels {
                id,
                dx: (quad[0].x - rect.left).round() as i32,
                dy: (quad[0].y - rect.top).round() as i32,
                copy: *copy,
            })
        }
        _ => None,
    })
}

/// Non-zero render key of a live pixel move (generation, page, bitmap, offset, mask).
pub fn live_pixels_key(
    generation: u64,
    page: usize,
    lp: &LivePixels,
    mask: &fp_raster::Mask,
) -> u64 {
    let mk = super::overlay::mask_key(mask);
    hash_ids(
        generation ^ 0x5049_5845_4c53,
        &[
            page as u64,
            lp.id.0,
            lp.dx as i64 as u64,
            lp.dy as i64 as u64,
            lp.copy as u64,
            mk.0 as u64,
            mk.1 as u64,
            mk.2 as u64,
            mk.3 as u64,
            mk.4 as u64,
        ],
    ) | 1
}

/// `doc` with the live pixel move applied (a copy; the session's document is not touched).
pub fn apply_live_pixels(
    doc: &Arc<Document>,
    page: usize,
    lp: &LivePixels,
    mask: &fp_raster::Mask,
) -> Arc<Document> {
    let mut out = (**doc).clone();
    if fp_engine::commands::preview_move_pixels(
        &mut out,
        page,
        lp.id,
        mask,
        (lp.dx, lp.dy),
        lp.copy,
    ) {
        Arc::new(out)
    } else {
        Arc::clone(doc)
    }
}

// ------------------------------------------- F141: whole-object move through the page pipeline

/// Most elements (with the children of groups) a live move renders through the page pipeline;
/// larger selections keep the sprite preview of BA-097 (PERF-3 fallback, F141).
pub const LIVE_MOVE_MAX_ELEMENTS: usize = 400;

/// A whole-object move drag with "Preview drag" on (RE spec 76 §7.1): the canvas image is the
/// document with the elements `ids` moved by `(dx, dy)` (or duplicated there with `copy`),
/// composed normally, so it equals the image after the release.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveMove {
    pub ids: Vec<ElementId>,
    pub dx: f32,
    pub dy: f32,
    pub copy: bool,
}

/// Elements in `e` (itself and the children of groups, masks and compound shapes), at most `cap`.
fn count_elements(e: &Element, cap: usize) -> usize {
    let mut n = 1;
    let add = |c: &Element, n: &mut usize| {
        if *n < cap {
            *n += count_elements(c, cap - *n);
        }
    };
    match &e.kind {
        fp_model::ElementKind::Group(g) => {
            for c in g.mask.iter().chain(&g.children) {
                add(c, &mut n);
            }
        }
        fp_model::ElementKind::CompoundShape(c) => {
            for o in c.result.iter().chain(&c.operands) {
                add(o, &mut n);
            }
        }
        _ => {}
    }
    n.min(cap)
}

/// The whole-object move drag in the tool feedback that renders through the page pipeline;
/// `None` without one, for scale/transform previews, pixel moves and selections over
/// [`LIVE_MOVE_MAX_ELEMENTS`] (those keep the sprites).
pub fn live_move(
    feedback: &[fp_engine::tools::Overlay],
    doc: &Document,
    page: usize,
) -> Option<LiveMove> {
    feedback.iter().find_map(|o| match o {
        fp_engine::tools::Overlay::Ghost {
            ids,
            copy,
            pixels: false,
            moved_by: Some(d),
            ..
        } => {
            if ids.is_empty() || ids.len() > LIVE_MOVE_MAX_ELEMENTS {
                return None;
            }
            let ids: Vec<ElementId> = ids.iter().map(|i| ElementId(*i)).collect();
            let cap = LIVE_MOVE_MAX_ELEMENTS + 1;
            let mut n = 0;
            for p in fp_engine::docops::find_many_on_page(doc, page, &ids)
                .iter()
                .flatten()
            {
                if let Some(e) = doc.element_at(p) {
                    n += count_elements(e, cap);
                }
                if n > LIVE_MOVE_MAX_ELEMENTS {
                    return None;
                }
            }
            Some(LiveMove {
                ids,
                dx: d.x,
                dy: d.y,
                copy: *copy,
            })
        }
        _ => None,
    })
}

/// Non-zero render key of a live move (generation, page, elements, offset, copy).
pub fn live_move_key(generation: u64, page: usize, lm: &LiveMove) -> u64 {
    let mut v: Vec<u64> = vec![
        page as u64,
        lm.dx.to_bits() as u64,
        lm.dy.to_bits() as u64,
        lm.copy as u64,
    ];
    v.extend(lm.ids.iter().map(|i| i.0));
    hash_ids(generation ^ 0x4d4f_5645, &v) | 1
}

/// `doc` with the live move applied like `edit.move` on release (a copy; the session's
/// document is not touched).
pub fn apply_live_move(doc: &Arc<Document>, page: usize, lm: &LiveMove) -> Arc<Document> {
    let mut out = (**doc).clone();
    if fp_engine::commands::preview_move(&mut out, page, &lm.ids, (lm.dx, lm.dy), lm.copy) {
        Arc::new(out)
    } else {
        Arc::clone(doc)
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
