//! Live-Vorschau der Malwerkzeuge (Pinsel, Bleistift, Radiergummi; PINSEL-1, N31): Die Engine
//! liefert die geänderten Pixel der Bitmap als Kacheln ([`fp_engine::paint::PaintPreview`]); hier
//! wird je Kachel eine Textur gehalten und nur neu hochgeladen, wenn sich die Kachel geändert hat.
//! Nach dem Loslassen bleibt die Vorschau stehen, bis das Hintergrundbild mit dem gemalten Strich
//! da ist (höchstens 2 s), damit der Strich nicht kurz verschwindet.
//!
//! BA-039: When the stroke paints an existing bitmap that the page draws directly (not inside a
//! symbol), the preview is composited in the bitmap's stacking position instead of lying above
//! everything ([`paint_composite`]): the stroke is applied to a scratch copy of the bitmap, and the
//! normal renderer draws the page with the bitmap's pixels replaced, so opacity, blend mode, live
//! effects, masks and the objects above apply. Only the composite tiles the stroke changed are
//! rendered again each frame (other objects come from the element cache). Otherwise the old
//! overlay ([`paint`]) is used.

use egui::{Color32, Pos2, Rect, TextureHandle, TextureOptions};
use fp_engine::paint::PaintPreview;
use fp_geom::IRect;
use fp_geom::Point;
use fp_raster::RgbaImage;
use std::collections::HashMap;
use std::sync::Arc;

/// Texturen der Vorschau eines Dokuments.
#[derive(Clone, Default)]
pub struct PaintPreviewCache {
    /// Kennung des Zugs, zu dem die Texturen gehören.
    stroke: u64,
    tiles: HashMap<(i32, i32), (TextureHandle, u64, fp_geom::IRect)>,
    /// Zeit, zu der der Zug zuletzt lief (Nachlauf nach dem Loslassen).
    last_seen: f64,
    /// Zug ist vorbei (Loslassen oder Esc).
    ended: bool,
    /// Composite mode (BA-039): scratch bitmap of the stroke.
    composite: Option<Box<Scratch>>,
}

/// Scratch raster of the painted bitmap during a composited stroke (BA-039).
#[derive(Clone)]
struct Scratch {
    target: fp_model::ElementId,
    /// Stored bitmap pixels when the stroke began.
    orig: Arc<RgbaImage>,
    /// The bitmap with the stroke so far.
    scratch: Arc<RgbaImage>,
    /// The bitmap has visible live effects (renders need the whole bitmap and a margin).
    fx: bool,
    /// Engine tile versions already applied to `scratch`.
    applied: HashMap<(i32, i32), u64>,
    /// Document state the composite tiles were rendered for.
    doc_generation: u64,
}

/// Side of a composite tile (document pixels).
const CT: i32 = 256;
/// Reach of live effects around a change (shadows, glows, blurs) in document pixels.
const FX_MARGIN: i32 = 48;

fn key(doc: fp_engine::DocId) -> egui::Id {
    egui::Id::new(("fp-paint-preview", doc.0))
}

fn color_image(img: &fp_raster::RgbaImage) -> egui::ColorImage {
    egui::ColorImage::from_rgba_unmultiplied([img.width(), img.height()], img.as_bytes())
}

fn draw(painter: &egui::Painter, c: &PaintPreviewCache, spp: &dyn Fn(Point) -> Pos2) {
    for (tex, _, r) in c.tiles.values() {
        let rect = Rect::from_min_max(
            spp(Point::new(r.left as f32, r.top as f32)),
            spp(Point::new(r.right as f32, r.bottom as f32)),
        );
        painter.image(
            tex.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    }
}

/// Zeichnet die Vorschau eines laufenden Zugs; lädt nur geänderte Kacheln hoch. Liefert die Zahl
/// der hochgeladenen Kacheln (Tests).
pub fn paint(
    ctx: &egui::Context,
    painter: &egui::Painter,
    doc: fp_engine::DocId,
    p: &PaintPreview,
    zoom: f64,
    time: f64,
    spp: &dyn Fn(Point) -> Pos2,
) -> usize {
    let mut uploads = 0;
    let filter = if zoom >= 1.0 {
        TextureOptions::NEAREST
    } else {
        TextureOptions::LINEAR
    };
    // Zwischenspeicher herausnehmen: Texturen anlegen sperrt den Kontext selbst
    let mut c = ctx
        .data_mut(|d| d.remove_temp::<PaintPreviewCache>(key(doc)))
        .unwrap_or_default();
    if c.stroke != p.stroke {
        c = PaintPreviewCache {
            stroke: p.stroke,
            ..Default::default()
        };
    }
    c.last_seen = time;
    c.ended = false;
    for t in p.tiles.iter() {
        match c.tiles.get_mut(&t.key) {
            Some((_, ver, _)) if *ver == t.version => {}
            Some((tex, ver, _)) => {
                tex.set(color_image(&t.image), filter);
                *ver = t.version;
                uploads += 1;
            }
            None => {
                let tex = ctx.load_texture(
                    format!("paint-{}-{}-{}", p.stroke, t.key.0, t.key.1),
                    color_image(&t.image),
                    filter,
                );
                c.tiles.insert(t.key, (tex, t.version, t.rect));
                uploads += 1;
            }
        }
    }
    draw(painter, &c, spp);
    ctx.data_mut(|d| d.insert_temp(key(doc), c));
    uploads
}

/// Nach dem Zug: Vorschau stehen lassen, bis das Hintergrundbild den Stand `generation` zeigt
/// (`shown_generation`), höchstens 2 s; sonst Texturen freigeben.
pub fn linger(
    ctx: &egui::Context,
    painter: &egui::Painter,
    doc: fp_engine::DocId,
    generation: u64,
    shown_generation: Option<u64>,
    time: f64,
    spp: &dyn Fn(Point) -> Pos2,
) {
    let Some(mut c) = ctx.data_mut(|d| d.remove_temp::<PaintPreviewCache>(key(doc))) else {
        return;
    };
    if c.tiles.is_empty() {
        return;
    }
    if !c.ended {
        c.ended = true;
        c.last_seen = time;
    }
    let waiting = time - c.last_seen < 2.0 && shown_generation.is_none_or(|g| g != generation);
    if waiting {
        draw(painter, &c, spp);
        ctx.data_mut(|d| d.insert_temp(key(doc), c));
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

/// Anzahl der gehaltenen Vorschau-Kacheln (Tests).
pub fn cached_tiles(ctx: &egui::Context, doc: fp_engine::DocId) -> usize {
    ctx.data(|d| {
        d.get_temp::<PaintPreviewCache>(key(doc))
            .map(|c| c.tiles.len())
            .unwrap_or(0)
    })
}

/// Inputs of a composited preview (BA-039).
pub struct CompositeInput<'a> {
    pub renderer: &'a fp_render::Renderer,
    pub doc: &'a fp_model::Document,
    /// Document state (`generation`): a new state starts the scratch raster again.
    pub generation: u64,
    pub page: usize,
    pub frame: usize,
    /// View > gamma preview applies to the composite like to the background image.
    pub gamma: bool,
}

/// Can the stroke `p` be composited in the bitmap's stacking position (BA-039)? Needs an existing
/// target bitmap that the page draws directly (not in a symbol).
pub fn composite_possible(doc: &fp_model::Document, page: usize, p: &PaintPreview) -> bool {
    p.target
        .is_some_and(|t| fp_render::find_bitmap(doc, page, fp_model::ElementId(t)).is_some())
}

/// Starts the scratch raster of a stroke on `target`.
fn begin(input: &CompositeInput, target: u64) -> Option<Scratch> {
    let (e, _) = fp_render::find_bitmap(input.doc, input.page, fp_model::ElementId(target))?;
    let fp_model::ElementKind::Bitmap(b) = &e.kind else {
        return None;
    };
    let orig = Arc::new(fp_engine::docops::bitmap_image(b)?);
    Some(Scratch {
        target: e.id,
        scratch: Arc::clone(&orig),
        orig,
        fx: fp_render::has_visible_effects(&e),
        applied: HashMap::new(),
        doc_generation: input.generation,
    })
}

/// Grows `img` so that it covers `r` (new area transparent).
fn grow(img: &mut Arc<RgbaImage>, r: IRect) {
    let b = img.bounds();
    let u = b.union(&r);
    if u == b {
        return;
    }
    let Ok(mut n) = RgbaImage::new(u) else { return };
    let ow = b.width() as usize;
    let uw = u.width() as usize;
    for y in b.top..b.bottom {
        let so = ((y - b.top) as usize) * ow;
        let d0 = ((y - u.top) as usize) * uw + (b.left - u.left) as usize;
        n.data_mut()[d0..d0 + ow].copy_from_slice(&img.data()[so..so + ow]);
    }
    *img = Arc::new(n);
}

/// Applies an engine tile to the scratch raster: shown pixels replace the bitmap (the true value
/// from `exact` where the tile shows a backdrop mix), transparent ones restore the original.
/// Returns the bounding box of the pixels that changed in the scratch raster.
fn apply_tile(sc: &mut Scratch, t: &fp_engine::paint::PaintTile) -> IRect {
    let b = sc.scratch.bounds();
    if t.rect.intersect(&b) != t.rect && !t.image.alpha_bounds().is_empty() {
        grow(&mut sc.scratch, t.rect);
    }
    let r = t.rect.intersect(&sc.scratch.bounds());
    if r.is_empty() {
        return r;
    }
    let s = Arc::make_mut(&mut sc.scratch);
    let exact = t.exact.as_deref();
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for y in r.top..r.bottom {
        for x in r.left..r.right {
            let v = t.image.get(x, y).unwrap_or([0; 4]);
            let px = if v[3] == 0 {
                sc.orig.get(x, y).unwrap_or([0; 4])
            } else {
                exact.and_then(|e| e.get(x, y)).unwrap_or(v)
            };
            if s.get(x, y) != Some(px) {
                s.set_pixel(x, y, fp_color::Rgba8::new(px[0], px[1], px[2], px[3]));
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
        }
    }
    if x0 > x1 {
        return IRect::default();
    }
    IRect::new(x0, y0, x1 + 1, y1 + 1)
}

/// Composite tiles (keys) touching `r`.
fn ctiles(r: IRect) -> Vec<(i32, i32)> {
    if r.is_empty() {
        return Vec::new();
    }
    let (x0, y0) = (r.left.div_euclid(CT), r.top.div_euclid(CT));
    let (x1, y1) = ((r.right - 1).div_euclid(CT), (r.bottom - 1).div_euclid(CT));
    (y0..=y1)
        .flat_map(move |y| (x0..=x1).map(move |x| (x, y)))
        .collect()
}

/// Draws the preview of a running stroke composited in the bitmap's stacking position (BA-039);
/// renders and uploads only the composite tiles the stroke changed since the last frame. `None`
/// when the target cannot be composited (the caller then uses [`paint`]); otherwise the number of
/// uploaded tiles.
pub fn paint_composite(
    ctx: &egui::Context,
    painter: &egui::Painter,
    doc: fp_engine::DocId,
    p: &PaintPreview,
    input: &CompositeInput,
    time: f64,
    spp: &dyn Fn(Point) -> Pos2,
) -> Option<usize> {
    let target = p.target?;
    let page_rect = fp_render::Renderer::page_rect(input.doc, input.page)?;
    let mut c = ctx
        .data_mut(|d| d.remove_temp::<PaintPreviewCache>(key(doc)))
        .unwrap_or_default();
    let fresh = c.stroke != p.stroke
        || c.composite
            .as_ref()
            .is_none_or(|s| s.target.0 != target || s.doc_generation != input.generation);
    if fresh {
        let Some(sc) = begin(input, target) else {
            ctx.data_mut(|d| d.insert_temp(key(doc), c));
            return None;
        };
        c = PaintPreviewCache {
            stroke: p.stroke,
            composite: Some(Box::new(sc)),
            ..Default::default()
        };
    }
    c.last_seen = time;
    c.ended = false;
    let uploads = update_composite(ctx, &mut c, p, input, page_rect);
    draw(painter, &c, spp);
    ctx.data_mut(|d| d.insert_temp(key(doc), c));
    uploads
}

/// Steps 1-3 of [`paint_composite`]: scratch raster, one render, tile textures.
fn update_composite(
    ctx: &egui::Context,
    c: &mut PaintPreviewCache,
    p: &PaintPreview,
    input: &CompositeInput,
    page_rect: IRect,
) -> Option<usize> {
    let sc = c.composite.as_deref_mut()?;
    // 1. apply changed engine tiles to the scratch raster
    let mut dirty = IRect::default();
    for t in p.tiles.iter() {
        if sc.applied.get(&t.key) == Some(&t.version) {
            continue;
        }
        let r = apply_tile(sc, t);
        sc.applied.insert(t.key, t.version);
        dirty = dirty.union(&r);
    }
    if dirty.is_empty() {
        return Some(0);
    }
    if sc.fx {
        dirty = IRect::new(
            dirty.left - FX_MARGIN,
            dirty.top - FX_MARGIN,
            dirty.right + FX_MARGIN,
            dirty.bottom + FX_MARGIN,
        );
    }
    // 2. one render of the changed rectangle
    let region = dirty.intersect(&page_rect);
    if region.is_empty() {
        return Some(0);
    }
    let image = if sc.fx {
        Arc::clone(&sc.scratch)
    } else {
        Arc::new(sc.scratch.cropped(region))
    };
    let ov = fp_render::PixelOverride {
        id: sc.target,
        image,
    };
    let out =
        input
            .renderer
            .render_region_with(input.doc, input.page, input.frame, region, 1.0, &ov)?;
    let mut img = out.image;
    img.set_origin(region.left, region.top);
    if input.gamma {
        super::render::apply_gamma_preview(&mut img);
    }
    // without effects, unchanged bitmap pixels stay transparent (the background image shows the
    // same there: no seams at the tile edges)
    if !sc.fx {
        for y in region.top..region.bottom {
            for x in region.left..region.right {
                if sc.scratch.get(x, y) == sc.orig.get(x, y) {
                    img.set_pixel(x, y, fp_color::Rgba8::new(0, 0, 0, 0));
                }
            }
        }
    }
    // 3. partial uploads into the composite tile textures (new tiles start transparent)
    let mut uploads = 0;
    for k in ctiles(region) {
        let tr = IRect::from_xywh(k.0 * CT, k.1 * CT, CT, CT).intersect(&page_rect);
        let sub = tr.intersect(&region);
        if sub.is_empty() {
            continue;
        }
        let part = color_image(&img.cropped(sub));
        let pos = [(sub.left - tr.left) as usize, (sub.top - tr.top) as usize];
        let entry = c.tiles.entry(k).or_insert_with(|| {
            let blank = egui::ColorImage::filled(
                [tr.width() as usize, tr.height() as usize],
                Color32::TRANSPARENT,
            );
            let tex = ctx.load_texture(
                format!("paint-c-{}-{}-{}", p.stroke, k.0, k.1),
                blank,
                TextureOptions::NEAREST,
            );
            (tex, 0, tr)
        });
        entry.0.set_partial(pos, part, TextureOptions::NEAREST);
        entry.1 += 1;
        uploads += 1;
    }
    Some(uploads)
}
