//! Miniaturen für Ebenen-, Seiten- und Inspektor-Vorschauen über fp-render. Zwischenspeicher je
//! (Dokument, Element/Seite, Größe); neu gerechnet wird nur, wenn sich genau dieses Element bzw.
//! diese Seite geändert hat (Copy-on-Write: anderer `Arc`), höchstens ein paar Miniaturen pro Bild
//! (Zähler je egui-Durchlauf; fehlende folgen in den nächsten Bildern).

use crate::app_state::AppState;
use egui::{ColorImage, Context, Id, TextureHandle, TextureOptions};
use fp_engine::DocId;
use fp_model::{Document, Element};
use fp_raster::RgbaImage;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Background render of a page thumbnail: generation and result channel.
type PendingThumb = (u64, std::sync::mpsc::Receiver<Option<RgbaImage>>);

#[derive(Default)]
struct Cache {
    map: HashMap<(u64, u64, u32), (u64, TextureHandle)>,
    frame: u64,
    used: u32,
    /// Seitenminiaturen, die im Hintergrund neu gerechnet werden (Schlüssel → Stand, Ergebnis).
    pending: HashMap<(u64, u64, u32), PendingThumb>,
}

const MAX_PER_FRAME: u32 = 6;

fn cache(ctx: &Context) -> Arc<Mutex<Cache>> {
    let id = Id::new("fp-thumb-cache");
    if let Some(c) = ctx.data(|d| d.get_temp::<Arc<Mutex<Cache>>>(id)) {
        return c;
    }
    let c = Arc::new(Mutex::new(Cache::default()));
    ctx.data_mut(|d| d.insert_temp(id, Arc::clone(&c)));
    c
}

fn to_texture(ctx: &Context, name: String, img: &RgbaImage, size: u32) -> Option<TextureHandle> {
    if img.width() == 0 || img.height() == 0 {
        return None;
    }
    let s = (size as f32 / img.width().max(img.height()) as f32).min(1.0);
    let (w, h) = (
        ((img.width() as f32 * s).round() as u32).max(1),
        ((img.height() as f32 * s).round() as u32).max(1),
    );
    let small = if s < 1.0 {
        fp_raster::resample::resize(img, w, h, fp_raster::resample::Sampler::Bilinear)
    } else {
        img.clone()
    };
    let ci = ColorImage::from_rgba_unmultiplied([small.width(), small.height()], small.as_bytes());
    Some(ctx.load_texture(name, ci, TextureOptions::LINEAR))
}

fn get(
    ctx: &Context,
    st: &AppState,
    key: (u64, u64, u32),
    generation: u64,
    render: impl FnOnce() -> Option<RgbaImage>,
) -> Option<TextureHandle> {
    let c = cache(ctx);
    let Ok(mut c) = c.lock() else { return None };
    let pass = ctx.cumulative_pass_nr();
    if c.frame != pass {
        c.frame = pass;
        c.used = 0;
    }
    let _ = st;
    let existing = c.map.get(&key).cloned();
    if let Some((g, t)) = &existing
        && *g == generation
    {
        return Some(t.clone());
    }
    if c.used >= MAX_PER_FRAME {
        ctx.request_repaint();
        return existing.map(|(_, t)| t);
    }
    c.used += 1;
    let img = render()?;
    let tex = to_texture(
        ctx,
        format!("thumb-{}-{}-{}", key.0, key.1, key.2),
        &img,
        key.2,
    )?;
    c.map.insert(key, (generation, tex.clone()));
    if c.map.len() > 2000 {
        c.map.clear();
    }
    Some(tex)
}

/// Wie [`get`], aber eine **vorhandene** Miniatur wird im Hintergrund erneuert; bis dahin bleibt die
/// alte stehen (PERF-1: die Seitenminiatur rechnet die ganze Seite 1:1 – bei Seiten mit Tausenden
/// Objekten nach jeder Pfeiltaste Hunderte ms im UI-Thread). Erste Miniatur und Tests
/// (`sync_render`) rechnen wie bisher sofort.
fn get_refresh_async(
    ctx: &Context,
    st: &AppState,
    key: (u64, u64, u32),
    generation: u64,
    render: impl FnOnce() -> Option<RgbaImage> + Send + 'static,
) -> Option<TextureHandle> {
    let c = cache(ctx);
    let Ok(mut guard) = c.lock() else { return None };
    // fertige Hintergrundrechnung übernehmen
    if let Some((g, rx)) = guard.pending.get(&key) {
        match rx.try_recv() {
            Ok(img) => {
                let g = *g;
                guard.pending.remove(&key);
                if let Some(tex) = img.and_then(|img| {
                    to_texture(
                        ctx,
                        format!("thumb-{}-{}-{}", key.0, key.1, key.2),
                        &img,
                        key.2,
                    )
                }) {
                    guard.map.insert(key, (g, tex));
                }
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                guard.pending.remove(&key);
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }
    let existing = guard.map.get(&key).cloned();
    match &existing {
        Some((g, t)) if *g == generation => return Some(t.clone()),
        Some(_) if !st.sync_render => {}
        _ => {
            drop(guard);
            return get(ctx, st, key, generation, render);
        }
    }
    guard.pending.entry(key).or_insert_with(|| {
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx2 = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(render());
            ctx2.request_repaint();
        });
        (generation, rx)
    });
    // läuft noch (ggf. für einen älteren Stand – danach startet die nächste Rechnung)
    ctx.request_repaint_after(std::time::Duration::from_millis(50));
    existing.map(|(_, t)| t)
}

/// Miniatur eines Elements.
pub fn element_thumb(
    ctx: &Context,
    st: &AppState,
    doc: DocId,
    generation: u64,
    d: &Arc<Document>,
    e: &Arc<Element>,
    size: u32,
) -> Option<TextureHandle> {
    let r = Arc::clone(&st.renderer);
    let frame = st.session.document(doc).map(|x| x.frame).unwrap_or(0);
    let _ = generation;
    let dep = if matches!(
        e.kind,
        fp_model::ElementKind::Instance(_) | fp_model::ElementKind::Group(_)
    ) {
        (Arc::as_ptr(d) as usize as u64).rotate_left(29)
    } else {
        0
    };
    get(
        ctx,
        st,
        (doc.0, e.id.0, size),
        (Arc::as_ptr(e) as usize as u64) ^ (frame as u64) ^ dep,
        || {
            let out = r.render_element(d, e, &fp_render::PointMap::identity(), frame);
            Some(out.image)
        },
    )
}

/// What the object thumbnail of a masked object shows (RE spec 73 §0.1, §7): with "Show fill and
/// stroke" on in the alpha channel / path outline mode the masked result including the underlay
/// (as on the canvas); with the underlay off, in the grayscale mode or with the mask disabled the
/// content alone, unmasked. Any other element is shown as it is. Mask editing changes nothing (the
/// reference's stretched content while a bitmap mask is edited is not copied).
pub fn thumb_subject(e: &Arc<Element>) -> Arc<Element> {
    use fp_model::{ElementKind, GroupKind};
    match &e.kind {
        ElementKind::Group(g)
            if g.kind.is_mask()
                && g.mask_flags.enabled
                && !(g.kind == GroupKind::VectorMask && !g.mask_flags.hide_attrs) =>
        {
            let mut c = (**e).clone();
            if let ElementKind::Group(g) = &mut c.kind {
                g.mask_flags.enabled = false;
            }
            Arc::new(c)
        }
        _ => Arc::clone(e),
    }
}

/// Image of the object thumbnail of a masked object: [`thumb_subject`] at its place on the page
/// (canvas-related framing like the mask thumbnail, transparent around it).
pub fn masked_object_image(
    r: &fp_render::Renderer,
    d: &Document,
    page: usize,
    e: &Arc<Element>,
    frame: usize,
    size: u32,
) -> Option<RgbaImage> {
    element_on_page_image(r, d, page, &thumb_subject(e), frame, size)
}

/// Image of the mask thumbnail (RE spec 73 §0.3): the mask as a grayscale image on black at its
/// place on the page (canvas-related). Alpha channel / path outline: the alpha of the mask object
/// → white; grayscale: Rec. 709 luma × alpha of its look (mask bitmaps are kept in display form,
/// white = visible, so an empty Reveal All mask is all white). Opaque, page format.
pub fn mask_image(
    r: &fp_render::Renderer,
    d: &Document,
    page: usize,
    group: &Element,
    mask: &Arc<Element>,
    frame: usize,
    size: u32,
) -> Option<RgbaImage> {
    let gray = matches!(&group.kind, fp_model::ElementKind::Group(g) if g.kind == fp_model::GroupKind::BitmapMask);
    let mut img = r
        .render_element(d, mask, &fp_render::PointMap::identity(), frame)
        .image;
    for p in img.data_mut() {
        let v = if gray {
            let l = fp_color::models::luma709(p[0], p[1], p[2]) as u32;
            ((l * p[3] as u32 + 127) / 255) as u8
        } else {
            p[3]
        };
        *p = [v, v, v, 255];
    }
    place_on_page(d, page, &img, size, fp_color::Rgba8::new(0, 0, 0, 255))
}

/// Miniatur des Objekts eines maskierten Objekts (Ebenen-Bedienfeld, [`masked_object_image`]).
/// Updated when the document changes, i.e. on stroke release, not during a stroke (the live
/// paint pixels are no part of the document).
pub fn masked_object_thumb(
    ctx: &Context,
    st: &AppState,
    doc: DocId,
    d: &Arc<Document>,
    e: &Arc<Element>,
    size: u32,
) -> Option<TextureHandle> {
    let r = Arc::clone(&st.renderer);
    let (frame, page) = st
        .session
        .document(doc)
        .map(|x| (x.frame, x.page))
        .unwrap_or((0, 0));
    let generation = (Arc::as_ptr(e) as usize as u64)
        ^ (frame as u64)
        ^ (Arc::as_ptr(d) as usize as u64).rotate_left(29)
        ^ page_geom(d, page);
    get(
        ctx,
        st,
        (doc.0, e.id.0 | MASKED_OBJECT, size),
        generation,
        || masked_object_image(&r, d, page, e, frame, size),
    )
}

/// Miniatur der Maske eines maskierten Objekts (Ebenen-Bedienfeld, [`mask_image`]): weiß =
/// sichtbar wie im Original, canvas-related; entering the mask edit changes nothing, a stroke
/// shows on release.
pub fn mask_thumb(
    ctx: &Context,
    st: &AppState,
    doc: DocId,
    d: &Arc<Document>,
    group: &fp_model::Element,
    mask: &Arc<Element>,
    size: u32,
) -> Option<TextureHandle> {
    let r = Arc::clone(&st.renderer);
    let (frame, page) = st
        .session
        .document(doc)
        .map(|x| (x.frame, x.page))
        .unwrap_or((0, 0));
    let gray = matches!(&group.kind, fp_model::ElementKind::Group(g) if g.kind == fp_model::GroupKind::BitmapMask);
    let generation = (Arc::as_ptr(mask) as usize as u64)
        ^ (frame as u64)
        ^ (u64::from(gray) << 63)
        ^ page_geom(d, page);
    get(
        ctx,
        st,
        (doc.0, mask.id.0 | MASK_IMAGE, size),
        generation,
        || mask_image(&r, d, page, group, mask, frame, size),
    )
}

/// Key part of the canvas of `page` (size and origin).
fn page_geom(d: &Document, page: usize) -> u64 {
    d.pages.get(page).map_or(0, |p| {
        let cv = p.canvas;
        (u64::from(cv.width.to_bits()) << 32 | u64::from(cv.height.to_bits()))
            ^ (u64::from(cv.origin.x.to_bits()) << 16)
            ^ u64::from(cv.origin.y.to_bits()).rotate_left(48)
    })
}

/// Schlüsselbit der Objektminiaturen maskierter Objekte.
const MASKED_OBJECT: u64 = 1 << 59;

/// Schlüsselbit der Maskenminiaturen.
const MASK_IMAGE: u64 = 1 << 61;

/// Schlüsselbit der Miniaturen „Element auf der Seite“ (getrennt von Element-, Seiten- und
/// Symbolminiaturen im selben Zwischenspeicher).
const ON_PAGE: u64 = 1 << 62;

/// Miniatur eines Elements an seiner Lage auf der verkleinerten Seite (Vorschaubild im Inspektor wie
/// im Original, PARITAET_A O2): transparentes Bild im Seitenformat (längste Seite `size`), darin nur
/// das Element an seinem Platz.
pub fn element_on_page_thumb(
    ctx: &Context,
    st: &AppState,
    doc: DocId,
    d: &Arc<Document>,
    page: usize,
    e: &Arc<Element>,
    size: u32,
) -> Option<TextureHandle> {
    let r = Arc::clone(&st.renderer);
    let frame = st.session.document(doc).map(|x| x.frame).unwrap_or(0);
    let cv = d.pages.get(page)?.canvas;
    let (pw, ph, ox, oy) = (cv.width, cv.height, cv.origin.x, cv.origin.y);
    let dep = if matches!(
        e.kind,
        fp_model::ElementKind::Instance(_) | fp_model::ElementKind::Group(_)
    ) {
        (Arc::as_ptr(d) as usize as u64).rotate_left(29)
    } else {
        0
    };
    let geom = (u64::from(pw.to_bits()) << 32 | u64::from(ph.to_bits()))
        ^ (u64::from(ox.to_bits()) << 16)
        ^ u64::from(oy.to_bits()).rotate_left(48);
    get(
        ctx,
        st,
        (doc.0, e.id.0 | ON_PAGE, size),
        (Arc::as_ptr(e) as usize as u64) ^ (frame as u64) ^ dep ^ geom,
        || element_on_page_image(&r, d, page, e, frame, size),
    )
}

/// Bild zu [`element_on_page_thumb`]: Seitenformat mit der längsten Seite `size` (höchstens 1:1),
/// transparent, das Element verkleinert an seiner Lage. `None` ohne Seite.
pub fn element_on_page_image(
    r: &fp_render::Renderer,
    d: &Document,
    page: usize,
    e: &Arc<Element>,
    frame: usize,
    size: u32,
) -> Option<RgbaImage> {
    let out = r.render_element(d, e, &fp_render::PointMap::identity(), frame);
    place_on_page(d, page, &out.image, size, fp_color::Rgba8::TRANSPARENT)
}

/// `img` (document coordinates) reduced onto the page format with the longest side `size` (at
/// most 1:1) at its place, over `base`. `None` without a page.
fn place_on_page(
    d: &Document,
    page: usize,
    img: &RgbaImage,
    size: u32,
    base: fp_color::Rgba8,
) -> Option<RgbaImage> {
    let cv = &d.pages.get(page)?.canvas;
    let (pw, ph, ox, oy) = (
        cv.width.max(1.0),
        cv.height.max(1.0),
        cv.origin.x,
        cv.origin.y,
    );
    let s = (size as f32 / pw.max(ph)).min(1.0);
    let (w, h) = (
        ((pw * s).round() as i32).max(1),
        ((ph * s).round() as i32).max(1),
    );
    let mut canvas = RgbaImage::filled(fp_raster::IRect::from_xywh(0, 0, w, h), base).ok()?;
    if !img.is_empty() {
        let b = img.bounds();
        let (x0, y0) = (
            ((b.left as f32 - ox) * s).floor() as i32,
            ((b.top as f32 - oy) * s).floor() as i32,
        );
        let (x1, y1) = (
            ((b.right as f32 - ox) * s).ceil() as i32,
            ((b.bottom as f32 - oy) * s).ceil() as i32,
        );
        let mut small = fp_raster::resample::resize(
            img,
            (x1 - x0).max(1) as u32,
            (y1 - y0).max(1) as u32,
            fp_raster::resample::Sampler::Bilinear,
        );
        small.set_origin(x0, y0);
        canvas.paste(&small);
    }
    Some(canvas)
}

/// Miniatur einer Seite (Status 0).
pub fn page_thumb(
    ctx: &Context,
    st: &AppState,
    doc: DocId,
    generation: u64,
    d: &Arc<Document>,
    page: usize,
    size: u32,
) -> Option<TextureHandle> {
    let r = Arc::clone(&st.renderer);
    let _ = generation;
    let key_gen = d
        .pages
        .get(page)
        .map(|p| Arc::as_ptr(p) as usize as u64)
        .unwrap_or(0)
        ^ ((Arc::as_ptr(d) as usize as u64).rotate_left(17)
            * u64::from(!d.symbols.is_empty() || !d.shared_layers.is_empty()));
    let d = Arc::clone(d);
    get_refresh_async(
        ctx,
        st,
        (doc.0, u64::MAX - page as u64, size),
        key_gen,
        move || {
            let out = r.render_page(&d, page, 0);
            Some(out.image)
        },
    )
}

/// Miniatur eines Symbols (Dokumentbibliothek).
pub fn symbol_thumb(
    ctx: &Context,
    st: &AppState,
    doc: DocId,
    generation: u64,
    d: &Arc<Document>,
    index: usize,
    size: u32,
) -> Option<TextureHandle> {
    let r = Arc::clone(&st.renderer);
    let sym = d.symbols.get(index).cloned()?;
    let _ = generation;
    get(
        ctx,
        st,
        (doc.0, (u64::MAX / 2) - index as u64, size),
        Arc::as_ptr(&sym) as usize as u64,
        || {
            let out = r.render_symbol(d, &sym, 0);
            Some(out.image)
        },
    )
}
